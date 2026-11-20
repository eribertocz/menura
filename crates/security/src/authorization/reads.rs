//! What each field and metric of the internal model reads: the inputs of the restrictions
//! (SCY-5.1.8) and the datasets a metric uses (E2-4.1, E2-4.3).

use std::collections::{BTreeMap, BTreeSet};

use menura_contracts::normalize_unquoted_identifier;
use menura_model::expression::{Expression, Identifier, Reference};
use menura_model::internal_model::{
    ContextModifier, Dataset, DeclaredAdditivity, FieldPath, InternalModel, Metric, NormalizedName,
};
use menura_representations::bound_query::MetricPath;

use crate::restrictions::RestrictionInput;
use crate::role::Role;
use crate::walk::children;

/// A source column of a dataset: the dataset, and the column name after the upper-case mapping
/// of E0D-1.1, which identifies one column of the catalog relation (E0D-1.5).
pub(crate) type ColumnKey = (NormalizedName, String);

/// What a restriction can protect: a field, or a source column a dataset metric reads directly
/// as `source.column`. A column is protected by every restriction that protects it (SCY-3.1a),
/// so it takes part in the restrictions as an item that reads that column only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ReadItem {
    /// A field.
    Field(FieldPath),
    /// A source column read directly.
    Column(ColumnKey),
}

/// What the fields and metrics of a model read.
pub(crate) struct ModelReads {
    /// The input of [`crate::restrictions::restrictions`].
    pub(crate) restriction_input: RestrictionInput<ReadItem, ColumnKey, MetricPath>,
    /// The datasets each metric reads itself, without those of its input metrics.
    pub(crate) metric_datasets: BTreeMap<MetricPath, BTreeSet<NormalizedName>>,
}

/// Everything the fields and metrics of `model` read.
pub(crate) fn model_reads(model: &InternalModel) -> ModelReads {
    let mut reads = ModelReads {
        restriction_input: RestrictionInput {
            field_columns: BTreeMap::new(),
            declared: declared_restrictions(model),
            metric_fields: BTreeMap::new(),
            metric_inputs: BTreeMap::new(),
        },
        metric_datasets: BTreeMap::new(),
    };
    for dataset in &model.parts().datasets {
        add_field_columns(&mut reads, dataset);
        for metric in &dataset.metrics {
            let path = MetricPath::Dataset {
                dataset: dataset.name.clone(),
                metric: metric.name.clone(),
            };
            add_metric(&mut reads, model, path, metric, Some(dataset));
        }
    }
    for metric in &model.parts().metrics {
        let path = MetricPath::SemanticModel(metric.name.clone());
        add_metric(&mut reads, model, path, metric, None);
    }
    add_key_columns(&mut reads, model);
    reads
}

/// The source column `column` of `dataset`, such as a relationship key column that is not a
/// field (E0D-1.5).
pub(crate) fn column_key(dataset: &NormalizedName, column: &NormalizedName) -> ColumnKey {
    (
        dataset.clone(),
        normalize_unquoted_identifier(column.as_str()),
    )
}

/// Every relationship key column that is not a field of its dataset reads that source column,
/// so the restrictions that protect the column protect the key (SCY-3.1a, E2-4.2).
fn add_key_columns(reads: &mut ModelReads, model: &InternalModel) {
    for relationship in &model.parts().relationships {
        let ends = [
            (&relationship.from, &relationship.from_columns),
            (&relationship.to, &relationship.to_columns),
        ];
        for (dataset, columns) in ends {
            let found = model.dataset(dataset);
            for column in columns {
                if found.is_some_and(|found| found.field(column).is_some()) {
                    continue;
                }
                let key = column_key(dataset, column);
                let item = ReadItem::Column(key.clone());
                let input = &mut reads.restriction_input;
                input.field_columns.insert(item, BTreeSet::from([key]));
            }
        }
    }
}

/// The name an identifier refers to (E0D-1.1): a regular one in upper case, a delimited one
/// exactly.
pub(crate) fn normalized(identifier: &Identifier) -> NormalizedName {
    if identifier.is_delimited() {
        NormalizedName::new(identifier.text())
    } else {
        NormalizedName::new(&normalize_unquoted_identifier(identifier.text()))
    }
}

/// Every field named in an `access.fields` item, with the roles of the item (SCY-3.1).
fn declared_restrictions(model: &InternalModel) -> Vec<(ReadItem, BTreeSet<Role>)> {
    let mut declared = Vec::new();
    for dataset in &model.parts().datasets {
        let items = dataset.access.iter().flat_map(|access| &access.fields);
        for item in items {
            let roles: BTreeSet<Role> = item
                .roles
                .iter()
                .map(|role| Role::new(role.as_str()))
                .collect();
            for field in &item.fields {
                let path = FieldPath {
                    dataset: dataset.name.clone(),
                    field: field.clone(),
                };
                declared.push((ReadItem::Field(path), roles.clone()));
            }
        }
    }
    declared
}

/// The source columns every field of `dataset` reads: the references of its expression
/// (E0D-3: a field expression reads source columns only).
fn add_field_columns(reads: &mut ModelReads, dataset: &Dataset) {
    for field in &dataset.fields {
        let columns: BTreeSet<ColumnKey> = field
            .expression
            .iter()
            .flat_map(references)
            .map(|reference| (dataset.name.clone(), column_text(&reference.name)))
            .collect();
        let path = FieldPath {
            dataset: dataset.name.clone(),
            field: field.name.clone(),
        };
        reads
            .restriction_input
            .field_columns
            .insert(ReadItem::Field(path), columns);
    }
}

/// The column a reference names, after the upper-case mapping of E0D-1.1 whatever its kind:
/// a delimited reference names the column spelled exactly as its text, and no two columns of
/// a relation have the same upper-case name (E0D-1.5).
fn column_text(identifier: &Identifier) -> String {
    normalize_unquoted_identifier(identifier.text())
}

/// What one metric reads directly, before its input metrics.
#[derive(Default)]
struct MetricReads {
    fields: BTreeSet<FieldPath>,
    columns: BTreeSet<ColumnKey>,
    inputs: BTreeSet<MetricPath>,
}

/// What `metric` reads: its fields, its source columns, its datasets and its input metrics.
fn add_metric(
    reads: &mut ModelReads,
    model: &InternalModel,
    path: MetricPath,
    metric: &Metric,
    scope: Option<&Dataset>,
) {
    let read = metric_reads(metric, scope);
    let mut datasets: BTreeSet<NormalizedName> = metric
        .classification
        .iter()
        .flat_map(|classification| classification.home_datasets.iter().cloned())
        .collect();
    datasets.extend(read.fields.iter().map(|field| field.dataset.clone()));
    datasets.extend(read.columns.iter().map(|(dataset, _)| dataset.clone()));
    datasets.extend(population_dataset(model, metric));
    let mut items: BTreeSet<ReadItem> = read.fields.into_iter().map(ReadItem::Field).collect();
    for column in read.columns {
        let item = ReadItem::Column(column.clone());
        let input = &mut reads.restriction_input;
        input
            .field_columns
            .insert(item.clone(), BTreeSet::from([column]));
        items.insert(item);
    }
    let input = &mut reads.restriction_input;
    input.metric_fields.insert(path.clone(), items);
    input.metric_inputs.insert(path.clone(), read.inputs);
    reads.metric_datasets.insert(path, datasets);
}

/// The fields, source columns and input metrics `metric` reads itself.
fn metric_reads(metric: &Metric, scope: Option<&Dataset>) -> MetricReads {
    let mut read = MetricReads::default();
    for node in metric.expression.iter().flat_map(nodes) {
        match node {
            Expression::Reference(reference) => {
                read_reference(reference, scope, &mut read.fields, &mut read.columns);
            }
            Expression::MetricAggregate(reference) => {
                read.inputs.insert(input_path(reference));
            }
            _ => {}
        }
    }
    add_declared_reads(metric, scope, &mut read.fields, &mut read.inputs);
    read
}

/// A reference of a metric expression: `dataset.field`, a bare field of the dataset of a
/// dataset metric, or `source.column`, a source column of that dataset (E0D-3).
fn read_reference(
    reference: &Reference,
    scope: Option<&Dataset>,
    fields: &mut BTreeSet<FieldPath>,
    columns: &mut BTreeSet<ColumnKey>,
) {
    let name = normalized(&reference.name);
    match (&reference.qualifier, scope) {
        (Some(qualifier), Some(dataset)) if normalized(qualifier).as_str() == "SOURCE" => {
            columns.insert((dataset.name.clone(), column_text(&reference.name)));
        }
        (Some(qualifier), _) => {
            fields.insert(FieldPath {
                dataset: normalized(qualifier),
                field: name,
            });
        }
        (None, Some(dataset)) => {
            fields.insert(FieldPath {
                dataset: dataset.name.clone(),
                field: name,
            });
        }
        (None, None) => {}
    }
}

/// The metric `AGG(reference)` reads: `AGG(metric)` a metric of the semantic model,
/// `AGG(dataset.metric)` a dataset metric.
fn input_path(reference: &Reference) -> MetricPath {
    let metric = normalized(&reference.name);
    match &reference.qualifier {
        Some(dataset) => MetricPath::Dataset {
            dataset: normalized(dataset),
            metric,
        },
        None => MetricPath::SemanticModel(metric),
    }
}

/// The fields and metrics the metric extensions read: the inputs of a derived metric, the base
/// and time field of a context metric, and the fields of a semi-additive declaration.
fn add_declared_reads(
    metric: &Metric,
    scope: Option<&Dataset>,
    fields: &mut BTreeSet<FieldPath>,
    inputs: &mut BTreeSet<MetricPath>,
) {
    if let Some(derived) = &metric.derived {
        inputs.extend(
            derived
                .inputs
                .iter()
                .cloned()
                .map(MetricPath::SemanticModel),
        );
    }
    if let Some(context) = &metric.context {
        inputs.insert(context_base(&context.base, scope));
        match &context.modifier {
            ContextModifier::PeriodToDate { time_dimension, .. }
            | ContextModifier::PriorPeriod { time_dimension, .. } => {
                fields.insert(time_dimension.clone());
            }
            ContextModifier::ShareOfTotal(_) => {}
        }
    }
    if let Some(DeclaredAdditivity::SemiAdditive(semi)) = &metric.declared_additivity {
        fields.insert(semi.over.clone());
        fields.extend(semi.entity.iter().cloned());
    }
}

/// The base of a context metric: a metric of its own dataset when it has one of that name,
/// otherwise a metric of the semantic model.
fn context_base(base: &NormalizedName, scope: Option<&Dataset>) -> MetricPath {
    match scope {
        Some(dataset) if dataset.metrics.iter().any(|metric| &metric.name == base) => {
            MetricPath::Dataset {
                dataset: dataset.name.clone(),
                metric: base.clone(),
            }
        }
        _ => MetricPath::SemanticModel(base.clone()),
    }
}

/// The dataset of the population a semi-additive metric expects (E2-4.1).
fn population_dataset(model: &InternalModel, metric: &Metric) -> Option<NormalizedName> {
    let Some(DeclaredAdditivity::SemiAdditive(semi)) = &metric.declared_additivity else {
        return None;
    };
    let population = semi.population.as_ref()?;
    model
        .parts()
        .datasets
        .iter()
        .find(|dataset| {
            dataset
                .populations
                .iter()
                .any(|item| &item.name == population)
        })
        .map(|dataset| dataset.name.clone())
}

/// The references of an expression, in written order.
fn references(expression: &Expression) -> Vec<&Reference> {
    nodes(expression)
        .into_iter()
        .filter_map(|node| match node {
            Expression::Reference(reference) => Some(reference),
            _ => None,
        })
        .collect()
}

/// Every node of an expression, in written order: each node before its subexpressions.
fn nodes(expression: &Expression) -> Vec<&Expression> {
    let mut found = vec![expression];
    for child in children(expression) {
        found.extend(nodes(child));
    }
    found
}
