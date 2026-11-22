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

/// The key of every dimension of `query`, in output order, over the columns of the `Scan` of
/// the join instance each dimension is bound to (E3-4.2a).
///
/// # Errors
///
/// An internal error for a field outside the instance its dimension is bound to.
pub(crate) fn dimension_keys(
    query: &BoundQuery,
    layout: &Layout<'_>,
) -> Result<Vec<NamedExpression>, PlanError> {
    query
        .columns()
        .dimensions()
        .enumerate()
        .map(|(position, dimension)| {
            let element = QueryElement::Dimension(position);
            let expression = dimension_key(dimension, &element, layout)
                .map_err(|violation| at(violation, &dimension.origin))?;
            Ok(NamedExpression {
                name: dimension_column(position),
                expression,
                derived: None,
            })
        })
        .collect()
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
