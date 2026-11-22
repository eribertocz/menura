//! The Aggregate keys: one per dimension, with its time grain (E4-4 step 6, QRY-4.1).

use menura_contracts::types::common::Grain;
use menura_model::expression::{DateUnit, Expression};
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind, BoundQuery};
use menura_representations::logical_plan::NamedExpression;
use menura_representations::origin::{Origin, QueryLocation};
use menura_representations::paths::QueryElement;

use crate::columns::{column_reference, dimension_column};
use crate::error::{InvariantViolation, PlanError};
use crate::layout::Layout;

/// The key of every dimension of `query`, in output order, over the columns of the `Scan` of
/// the join instance each dimension is bound to (E3-4.2a).
///
/// # Errors
///
/// An internal error for a dimension of the SQL form or a field outside its instance.
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
    let BoundDimensionKind::Field {
        field, time_grain, ..
    } = &dimension.kind
    else {
        return Err(InvariantViolation::ExpressionDimension);
    };
    let column = column_reference(layout.element_column(element, field)?.as_str())?;
    Ok(match time_grain {
        Some(grain) => Expression::DateTrunc {
            unit: grain_unit(*grain),
            operand: Box::new(column),
        },
        None => column,
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
