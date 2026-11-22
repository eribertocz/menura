//! SUM-3.2 and SUM-3.2a: the branches and summaries that never use summaries.

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::LogicalType;
use menura_expressions::typing::{Aggregates, Stop, Typed, TypingScope, type_expression};
use menura_model::expression::{AggregateCall, Expression, Reference};
use menura_model::internal_model::{
    Dataset, Field, InternalModel, NormalizedName, Summary, ValueType,
};
use menura_representations::logical_plan::{AggregateComponent, OutputSchema};

use menura_representations::bound_query::BoundQuery;

use super::branch::BranchShape;
use super::expression_map::{any_node, field_name, references};
use super::restricted::reads_restricted_metric;

/// Whether the branch is excluded by SUM-3.2: it still has a `Deduplicate`, one of its
/// components is read by a non-additive or semi-additive metric of the query,
/// or one of its components aggregates a float.
pub(super) fn excluded(shape: &BranchShape<'_>, model: &InternalModel, query: &BoundQuery) -> bool {
    shape.deduplicate
        || reads_restricted_metric(shape, model, query)
        || shape
            .components
            .iter()
            .any(|component| aggregates_float(component, shape.scan.schema()))
}

/// Whether the argument of `component` may be a float: a recomposed float sum can differ
/// from the base (SUM-3.2). The argument is typed by E0F-5a over the columns of the home
/// `Scan`; an argument with no known type is excluded too.
fn aggregates_float(component: &AggregateComponent, schema: &OutputSchema) -> bool {
    let argument = match &component.call {
        AggregateCall::Simple { argument, .. } | AggregateCall::Distinct { argument, .. } => {
            argument.as_ref()
        }
        AggregateCall::CountRows | AggregateCall::Percentile { .. } => return false,
    };
    let resolve = |reference: &Reference| {
        schema
            .columns()
            .iter()
            .find(|column| {
                reference.qualifier.is_none() && column.name.as_str() == reference.name.text()
            })
            .map(|column| Typed::value(column.logical_type.clone(), column.nullable))
            .ok_or(Stop::ReportedElsewhere)
    };
    let scope = TypingScope::new(&resolve, Aggregates::Allowed);
    type_expression(argument, &scope)
        .ok()
        .and_then(|typed| typed.logical().cloned())
        .is_none_or(|logical_type| is_float(&logical_type))
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
    /// `timestamp` with time zone. E0 stores the reference as the delimited identifier of its
    /// normalized text, so the column is the one whose normalized name equals it (CAT-8.2).
    fn is_zoned_column(&self, reference: &Reference) -> bool {
        self.catalog
            .relations
            .iter()
            .filter(|relation| relation.source == self.base.source)
            .flat_map(|relation| relation.columns.iter())
            .any(|column| {
                normalize_unquoted_identifier(&column.name)
                    == normalize_unquoted_identifier(reference.name.text())
                    && is_zoned(&column.logical_type)
            })
    }
}
