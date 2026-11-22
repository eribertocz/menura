//! The Aggregate keys: one per dimension, with its time grain (E4-4 step 6, QRY-4.1).

use menura_contracts::types::common::Grain;
use menura_model::expression::{DateUnit, Expression};
use menura_model::internal_model::{FieldPath, NormalizedName};
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind, BoundQuery};
use menura_representations::logical_plan::NamedExpression;
use menura_representations::origin::{Origin, QueryLocation};
use menura_representations::paths::QueryElement;

use crate::columns::{column_reference, dimension_column, field_name};
use crate::error::{InvariantViolation, PlanError};
use crate::layout::Layout;
use crate::rewrite::rewrite;

/// The expression dimensions of the SQL form, computed over the rows of a branch by a
/// `Project` whose origin is the query, so that their literals are bound parameters (E6-4.2).
#[derive(Clone)]
pub(crate) struct ComputedDimensions {
    /// Each expression dimension over the columns of the `Scan` of the join instance it is
    /// bound to, under the synthesized column its Aggregate key reads.
    pub(crate) expressions: Vec<NamedExpression>,
    /// Where the first of them is written.
    pub(crate) origin: QueryLocation,
}

/// The Aggregate keys of a branch.
pub(crate) struct BranchKeys {
    /// The key of every dimension, in output order: over the columns of the `Scan` of the join
    /// instance each dimension is bound to (E3-4.2a), or the column of a computed dimension.
    pub(crate) keys: Vec<NamedExpression>,
    /// The expression dimensions; `None` when the query has none.
    pub(crate) computed: Option<ComputedDimensions>,
}

/// The keys of every dimension of `query`, in output order. An expression dimension of the
/// SQL form is computed under a column synthesized from `menura.dimension_<n>`, which its key
/// reads.
///
/// # Errors
///
/// An internal error for a field outside the instance its dimension is bound to.
pub(crate) fn dimension_keys(
    query: &BoundQuery,
    layout: &Layout<'_>,
) -> Result<BranchKeys, PlanError> {
    let mut keys = Vec::new();
    let mut computed: Option<ComputedDimensions> = None;
    for (position, dimension) in query.columns().dimensions().enumerate() {
        let element = QueryElement::Dimension(position);
        let fail = |violation| at(violation, &dimension.origin);
        let mut expression = dimension_key(dimension, &element, layout).map_err(fail)?;
        if let BoundDimensionKind::Expression { .. } = dimension.kind {
            let column = layout.synthesized(&format!("menura.dimension_{}", position + 1));
            let reference = column_reference(column.as_str()).map_err(fail)?;
            let named = NamedExpression {
                name: column,
                expression: std::mem::replace(&mut expression, reference),
                derived: None,
            };
            computed
                .get_or_insert_with(|| ComputedDimensions {
                    expressions: Vec::new(),
                    origin: dimension.origin.clone(),
                })
                .expressions
                .push(named);
        }
        keys.push(NamedExpression {
            name: dimension_column(position),
            expression,
            derived: None,
        });
    }
    Ok(BranchKeys { keys, computed })
}

fn dimension_key(
    dimension: &BoundDimension,
    element: &QueryElement,
    layout: &Layout<'_>,
) -> Result<Expression, InvariantViolation> {
    match &dimension.kind {
        BoundDimensionKind::Field {
            field, time_grain, ..
        } => field_key(field, *time_grain, element, layout),
        BoundDimensionKind::Expression {
            dataset,
            expression,
        } => expression_key(dataset, expression, element, layout),
    }
}

/// The key of a field dimension: the column of its field, truncated to its time grain.
fn field_key(
    field: &FieldPath,
    time_grain: Option<Grain>,
    element: &QueryElement,
    layout: &Layout<'_>,
) -> Result<Expression, InvariantViolation> {
    let column = column_reference(layout.element_column(element, field)?.as_str())?;
    Ok(match time_grain {
        Some(grain) => Expression::DateTrunc {
            unit: grain_unit(grain),
            operand: Box::new(column),
        },
        None => column,
    })
}

/// The key of a dimension of the SQL form (E1-4.1, E2-3.3): its expression, with every field
/// reference turned into the reference to the column of that field. E2 qualifies each field
/// with its dataset, which must be the dimension's `dataset`.
fn expression_key(
    dataset: &NormalizedName,
    expression: &Expression,
    element: &QueryElement,
    layout: &Layout<'_>,
) -> Result<Expression, InvariantViolation> {
    rewrite(expression, &mut |node| {
        let Expression::Reference(reference) = node else {
            return Ok(None);
        };
        let field = FieldPath {
            dataset: reference
                .qualifier
                .as_ref()
                .map_or_else(|| dataset.clone(), field_name),
            field: field_name(&reference.name),
        };
        if &field.dataset != dataset {
            return Err(InvariantViolation::ForeignField);
        }
        let column = layout.element_column(element, &field)?;
        column_reference(column.as_str()).map(Some)
    })
}

/// The unit a time grain truncates to; E6 truncates in the query time zone (E6-5.2).
pub(crate) fn grain_unit(grain: Grain) -> DateUnit {
    match grain {
        Grain::Day => DateUnit::Day,
        Grain::Week => DateUnit::Week,
        Grain::Month => DateUnit::Month,
        Grain::Quarter => DateUnit::Quarter,
        Grain::Year => DateUnit::Year,
    }
}

/// The internal error `violation` at the query element `location`.
pub(crate) fn at(violation: InvariantViolation, location: &QueryLocation) -> PlanError {
    PlanError::invariant(violation, &Origin::Query(location.clone()))
}
