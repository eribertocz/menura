//! The integrity checks the model needs, recorded as data (E0H-4.3), with the definition hash
//! of each (CHK-3). Their SQL is generated later (E0H-4.2).

use semantic_compiler_contracts::{Sha256Hash, hash_value};
use semantic_compiler_model::internal_model::{
    CheckKind, Dataset, DeclaredAdditivity, Field, Metric, NormalizedName, Population,
    RecordedCheck, Relationship, Summary,
};
use serde_json::Value as Json;

use super::documents::{
    field_definition, metric_definition, object, population, relationship_definition, text,
};
use super::index::ReadNames;

/// The parts of the model the checks read.
pub(super) struct CheckedModel<'model> {
    /// The datasets, ordered by name.
    pub(super) datasets: &'model [Dataset],
    /// The relationships, ordered by name.
    pub(super) relationships: &'model [Relationship],
    /// The metrics of the semantic model, ordered by name.
    pub(super) metrics: &'model [Metric],
    /// The summaries.
    pub(super) summaries: &'model [Summary],
}

/// Every recorded check, sorted by `check_id` (E0H-4.3).
pub(super) fn recorded_checks(model: &CheckedModel<'_>) -> Vec<RecordedCheck> {
    let mut checks = Vec::new();
    for relationship in model
        .relationships
        .iter()
        .filter(|relationship| relationship.query_errors.is_empty())
    {
        let definition = relationship_check(model, relationship);
        for kind in [CheckKind::KeyUniqueness, CheckKind::OrphanKeys] {
            checks.push(check(kind, relationship.name.as_str(), &definition));
        }
    }
    for summary in model.summaries {
        checks.push(recorded(
            CheckKind::Summary,
            summary.dataset.as_str(),
            summary.definition_hash.clone(),
        ));
    }
    for dataset in model
        .datasets
        .iter()
        .filter(|dataset| dataset.query_errors.is_empty())
    {
        for declared in &dataset.populations {
            let definition = population_check(dataset, declared);
            checks.push(check(
                CheckKind::Population,
                declared.name.as_str(),
                &definition,
            ));
        }
    }
    for metric in model.metrics {
        checks.extend(semi_additive(model, None, metric));
    }
    for dataset in model.datasets {
        for metric in &dataset.metrics {
            checks.extend(semi_additive(model, Some(&dataset.name), metric));
        }
    }
    checks.sort_by(|left, right| left.check_id.cmp(&right.check_id));
    checks
}

/// The `semi_additive` check of `metric` when it is usable and semi-additive; a dataset metric
/// of `dataset` has the target `<dataset>.<metric>` (E0H-4.3).
fn semi_additive(
    model: &CheckedModel<'_>,
    dataset: Option<&NormalizedName>,
    metric: &Metric,
) -> Option<RecordedCheck> {
    let semi_additive = matches!(
        metric.declared_additivity,
        Some(DeclaredAdditivity::SemiAdditive(_))
    );
    if !semi_additive || !metric.query_errors.is_empty() {
        return None;
    }
    let target = match dataset {
        Some(dataset) => format!("{}.{}", dataset.as_str(), metric.name.as_str()),
        None => metric.name.as_str().to_owned(),
    };
    let definition = semi_additive_check(model, dataset, metric);
    Some(check(CheckKind::SemiAdditive, &target, &definition))
}

/// A check whose definition hash is the CON-2 hash of `{kind, target, definition}` (CHK-3).
fn check(kind: CheckKind, target: &str, definition: &Json) -> RecordedCheck {
    let hashed = object([
        ("kind", text(kind.as_str())),
        ("target", text(target)),
        ("definition", definition.clone()),
    ]);
    recorded(kind, target, hash_value(&hashed))
}

fn recorded(kind: CheckKind, target: &str, definition_hash: Sha256Hash) -> RecordedCheck {
    RecordedCheck {
        check_id: format!("{}:{target}", kind.as_str()),
        kind,
        target: target.to_owned(),
        definition_hash,
    }
}

fn dataset<'model>(model: &CheckedModel<'model>, name: &NormalizedName) -> Option<&'model Dataset> {
    model.datasets.iter().find(|dataset| dataset.name == *name)
}

/// The definitions of the fields `names` of `dataset`, ordered by name, each once.
fn fields<'name>(
    dataset: Option<&Dataset>,
    names: impl IntoIterator<Item = &'name NormalizedName>,
) -> Json {
    let mut names: Vec<&NormalizedName> = names.into_iter().collect();
    names.sort();
    names.dedup();
    let found: Vec<&Field> = names
        .into_iter()
        .filter_map(|name| dataset.and_then(|dataset| dataset.field(name)))
        .collect();
    Json::Array(found.into_iter().map(field_definition).collect())
}

fn source(dataset: Option<&Dataset>) -> Json {
    dataset.map_or(Json::Null, |dataset| text(&dataset.source))
}

/// What `key_uniqueness` and `orphan_keys` read: the relationship, the `source` of both
/// datasets and the fields of the column pairs (CHK-3).
fn relationship_check(model: &CheckedModel<'_>, relationship: &Relationship) -> Json {
    let from = dataset(model, &relationship.from);
    let to = dataset(model, &relationship.to);
    object([
        ("relationship", relationship_definition(relationship)),
        ("from_source", source(from)),
        ("to_source", source(to)),
        ("from_fields", fields(from, &relationship.from_columns)),
        ("to_fields", fields(to, &relationship.to_columns)),
    ])
}

/// What `population` reads: the population, the `source` of its dataset and every field it
/// reads (CHK-3).
fn population_check(dataset: &Dataset, declared: &Population) -> Json {
    let mut read: Vec<NormalizedName> = declared.named_fields().into_iter().cloned().collect();
    if let Some(condition) = &declared.required_when {
        read.extend(
            ReadNames::of(&condition.json_form())
                .references
                .into_iter()
                .map(|(_, field)| NormalizedName::new(field.as_str())),
        );
    }
    object([
        ("population", population(declared)),
        ("source", text(&dataset.source)),
        ("fields", fields(Some(dataset), &read)),
    ])
}

/// What `semi_additive` reads: the metric with its additivity extension, the `source` of its
/// dataset and every field it reads (CHK-3).
///
/// A model metric reads `dataset.field`; a dataset metric of `home` reads its own fields with
/// bare names, and catalog columns as `source.column`, which are not fields.
fn semi_additive_check(
    model: &CheckedModel<'_>,
    home: Option<&NormalizedName>,
    metric: &Metric,
) -> Json {
    let mut read: Vec<(NormalizedName, NormalizedName)> = Vec::new();
    if let Some(tree) = &metric.expression {
        for (qualifier, field) in ReadNames::of(&tree.json_form()).references {
            let dataset = match (home, qualifier) {
                (Some(home), None) => home.clone(),
                (None, Some(qualifier)) => NormalizedName::new(qualifier.as_str()),
                _ => continue,
            };
            read.push((dataset, NormalizedName::new(field.as_str())));
        }
    }
    if let Some(DeclaredAdditivity::SemiAdditive(keys)) = &metric.declared_additivity {
        for path in std::iter::once(&keys.over).chain(&keys.entity) {
            read.push((path.dataset.clone(), path.field.clone()));
        }
    }
    let mut datasets: Vec<&NormalizedName> = read.iter().map(|(dataset, _)| dataset).collect();
    datasets.sort();
    datasets.dedup();
    let read_from = datasets
        .into_iter()
        .map(|name| {
            let found = dataset(model, name);
            let names = read
                .iter()
                .filter(|(dataset, _)| dataset == name)
                .map(|(_, field)| field);
            object([
                ("dataset", text(name.as_str())),
                ("source", source(found)),
                ("fields", fields(found, names)),
            ])
        })
        .collect();
    object([
        ("metric", metric_definition(metric)),
        ("datasets", Json::Array(read_from)),
    ])
}
