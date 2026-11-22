//! SUM-3.2 and SUM-3.2a: the branches and summaries that never use summaries.

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::LogicalType;
use menura_model::expression::{
    AggregateCall, BinaryOperator, Expression, Literal, Reference, SimpleAggregate,
};
use menura_model::internal_model::{
    Additivity, Dataset, Field, InternalModel, Metric, NormalizedName, Summary, ValueType,
};
use menura_representations::logical_plan::{AggregateComponent, OutputSchema};

use super::branch::BranchShape;
use super::equivalence::{ScanAggregate, over_scan};
use super::expression_map::{any_node, columns_read, field_name, references};

/// Whether the branch is excluded by SUM-3.2: it still has a `Deduplicate`, one of its
/// components is a component of a non-additive or semi-additive metric of its home dataset,
/// or one of its components aggregates a float.
pub(super) fn excluded(shape: &BranchShape<'_>, model: &InternalModel, summary: &Summary) -> bool {
    let summary_dataset = model.dataset(&summary.dataset);
    shape.deduplicate
        || reads_restricted_metric(shape, model)
        || shape.components.iter().any(|component| {
            aggregates_float(component, shape.scan.schema(), summary, summary_dataset)
        })
}

/// Whether a component of the branch is, after normalization, a component of a non-additive
/// or semi-additive metric of its home dataset. A component shared with an additive metric is
/// excluded too: when in doubt, the base is used (SUM-1).
fn reads_restricted_metric(shape: &BranchShape<'_>, model: &InternalModel) -> bool {
    let parts = model.parts();
    let metrics = parts.metrics.iter().chain(
        parts
            .datasets
            .iter()
            .flat_map(|dataset| dataset.metrics.iter()),
    );
    let restricted: Vec<ScanAggregate> = metrics
        .filter(|metric| is_restricted(metric))
        .filter_map(|metric| metric.classification.as_ref())
        .flat_map(|classification| classification.components.iter())
        .filter(|component| &component.home_dataset == shape.home)
        .filter_map(|component| {
            over_scan(&Expression::Aggregate {
                call: component.call.clone(),
                filter: component.filter.clone().map(Box::new),
            })
        })
        .collect();
    shape.components.iter().any(|component| {
        restricted
            .iter()
            .any(|(call, filter)| call == &component.call && filter == &component.filter)
    })
}

/// Whether `metric` is declared or deduced non-additive or semi-additive (E4-7.9, ADD-3).
fn is_restricted(metric: &Metric) -> bool {
    metric.declared_additivity.is_some()
        || metric
            .classification
            .as_ref()
            .is_some_and(|classification| classification.additivity != Additivity::Additive)
}

/// Whether the argument of `component` may be a float: a recomposed float sum can differ
/// from the base (SUM-3.2). A plain column has the type of its `Scan` column. Otherwise the
/// type is that of a `SUM`, `MIN` or `MAX` measure of the same argument, whose summary field
/// has the result type of its `base_aggregate` (E0C-57, E0F-5). Otherwise an argument that
/// reads a float column or literal, or uses a cast, a division or a function, may be a
/// float: when in doubt, the base is used (SUM-1).
fn aggregates_float(
    component: &AggregateComponent,
    schema: &OutputSchema,
    summary: &Summary,
    summary_dataset: Option<&Dataset>,
) -> bool {
    let argument = match &component.call {
        AggregateCall::Simple { argument, .. } | AggregateCall::Distinct { argument, .. } => {
            argument.as_ref()
        }
        AggregateCall::CountRows | AggregateCall::Percentile { .. } => return false,
    };
    let column_type = |name: &str| {
        schema
            .columns()
            .iter()
            .find(|column| column.name.as_str() == name)
            .map(|column| &column.logical_type)
    };
    if let Some(column) = columns_read(argument).first()
        && matches!(argument, Expression::Reference(_))
    {
        return column_type(column.as_str()).is_none_or(is_float);
    }
    if let Some(measured) = measured_type(argument, summary, summary_dataset) {
        return is_float(&measured);
    }
    any_node(argument, &mut |node| match node {
        Expression::Reference(reference) => {
            reference.qualifier.is_some() || column_type(reference.name.text()).is_none_or(is_float)
        }
        Expression::Literal(Literal::Float(_))
        | Expression::Cast { .. }
        | Expression::Function { .. }
        | Expression::Binary {
            operator: BinaryOperator::Divide,
            ..
        } => true,
        _ => false,
    })
}

/// The type of `argument` read from the summary field of a `SUM`, `MIN` or `MAX` measure
/// of it, whose result type is a float exactly when its argument is (E0F-5).
fn measured_type(
    argument: &Expression,
    summary: &Summary,
    summary_dataset: Option<&Dataset>,
) -> Option<LogicalType> {
    let dataset = summary_dataset?;
    summary.measures.iter().find_map(|measure| {
        let (call, _) = over_scan(measure.base_aggregate.as_ref()?)?;
        let AggregateCall::Simple {
            function: SimpleAggregate::Sum | SimpleAggregate::Minimum | SimpleAggregate::Maximum,
            argument: measured,
        } = call
        else {
            return None;
        };
        if measured.as_ref() != argument {
            return None;
        }
        match &dataset.field(&measure.field)?.typed.as_ref()?.value_type {
            ValueType::Logical(logical_type) => Some(logical_type.clone()),
            ValueType::Null => None,
        }
    })
}

fn is_float(logical_type: &LogicalType) -> bool {
    matches!(logical_type, LogicalType::Float { .. })
}

/// Whether a summary row depends on the time or time zone of its build (SUM-3.2a): a
/// dimension `base` field or a field a `base_aggregate` reads uses `CURRENT_*`, directly or
/// through other fields, or reads a `timestamp` with time zone other than as the plain column
/// of a time dimension.
pub(super) fn depends_on_build_time(
    summary: &Summary,
    base: &Dataset,
    catalog: &CatalogSchemaDocument,
) -> bool {
    let reader = FieldReader { base, catalog };
    let dimensions = summary.dimensions.iter().any(|dimension| {
        base.field(&dimension.base).is_none_or(|field| {
            let plain_time = dimension.grain.is_some() && reader.is_plain_column(field);
            reader.reads_now(field, &mut Vec::new())
                || (!plain_time && reader.reads_zoned(field, &mut Vec::new()))
        })
    });
    let measures = summary.measures.iter().any(|measure| {
        let Some(tree) = &measure.base_aggregate else {
            return false;
        };
        reads_now(tree)
            || references(tree).iter().any(|reference| {
                base.field(&NormalizedName::new(&field_name(&reference.name)))
                    .is_none_or(|field| {
                        reader.reads_now(field, &mut Vec::new())
                            || reader.reads_zoned(field, &mut Vec::new())
                    })
            })
    });
    dimensions || measures
}

/// Whether `expression` uses `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP`.
fn reads_now(expression: &Expression) -> bool {
    any_node(expression, &mut |node| {
        matches!(
            node,
            Expression::CurrentDate | Expression::CurrentTime | Expression::CurrentTimestamp
        )
    })
}

fn is_zoned(logical_type: &LogicalType) -> bool {
    matches!(
        logical_type,
        LogicalType::Timestamp {
            with_time_zone: true,
            ..
        }
    )
}

/// Reads the fields of the base dataset and the catalog columns of its relation.
struct FieldReader<'input> {
    base: &'input Dataset,
    catalog: &'input CatalogSchemaDocument,
}

impl FieldReader<'_> {
    /// The other field of the base that `reference` in the expression of `field` names.
    fn other_field(&self, field: &Field, reference: &Reference) -> Option<&Field> {
        let name = NormalizedName::new(&field_name(&reference.name));
        self.base
            .field(&name)
            .filter(|other| other.name != field.name)
    }

    /// Whether `field` is a plain catalog column: its expression is one reference to a column
    /// of the relation.
    fn is_plain_column(&self, field: &Field) -> bool {
        matches!(&field.expression, Some(Expression::Reference(reference))
            if self.other_field(field, reference).is_none())
    }

    fn reads_now(&self, field: &Field, visited: &mut Vec<NormalizedName>) -> bool {
        if visited.contains(&field.name) {
            return false;
        }
        visited.push(field.name.clone());
        let Some(expression) = &field.expression else {
            return false;
        };
        reads_now(expression)
            || references(expression).iter().any(|reference| {
                self.other_field(field, reference)
                    .is_some_and(|other| self.reads_now(other, visited))
            })
    }

    fn reads_zoned(&self, field: &Field, visited: &mut Vec<NormalizedName>) -> bool {
        if visited.contains(&field.name) {
            return false;
        }
        visited.push(field.name.clone());
        let own = field.typed.as_ref().is_some_and(|typed| {
            matches!(&typed.value_type, ValueType::Logical(logical_type) if is_zoned(logical_type))
        });
        let Some(expression) = &field.expression else {
            return own;
        };
        own || references(expression).iter().any(|reference| {
            match self.other_field(field, reference) {
                Some(other) => self.reads_zoned(other, visited),
                None => self.is_zoned_column(reference),
            }
        })
    }

    /// Whether the catalog column `reference` names in the relation of the base is a
    /// `timestamp` with time zone.
    fn is_zoned_column(&self, reference: &Reference) -> bool {
        self.catalog
            .relations
            .iter()
            .filter(|relation| relation.source == self.base.source)
            .flat_map(|relation| relation.columns.iter())
            .any(|column| column.name == reference.name.text() && is_zoned(&column.logical_type))
    }
}
