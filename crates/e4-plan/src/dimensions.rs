//! The Aggregate keys: one per dimension, with its time grain (E4-4 step 6, QRY-4.1).

use menura_contracts::types::common::Grain;
use menura_model::expression::{DateUnit, Expression};
use menura_model::internal_model::{Dataset, FieldPath};
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind, BoundQuery};
use menura_representations::logical_plan::NamedExpression;
use menura_representations::origin::{Origin, QueryLocation};

use crate::columns::{column_reference, dimension_column};
use crate::error::{InvariantViolation, PlanError};

/// The key of every dimension of `query`, in output order, over the `Scan` columns of
/// `home`.
///
/// # Errors
///
/// An internal error for a dimension of the SQL form or a field outside `home`.
pub(crate) fn dimension_keys(
    query: &BoundQuery,
    home: &Dataset,
) -> Result<Vec<NamedExpression>, PlanError> {
    query
        .columns()
        .dimensions()
        .enumerate()
        .map(|(position, dimension)| {
            let expression = dimension_key(dimension, home)
                .map_err(|violation| at(violation, &dimension.origin))?;
            Ok(NamedExpression {
                name: dimension_column(position),
                expression,
            })
        })
        .collect()
}

fn dimension_key(
    dimension: &BoundDimension,
    home: &Dataset,
) -> Result<Expression, InvariantViolation> {
    let BoundDimensionKind::Field { field, time_grain } = &dimension.kind else {
        return Err(InvariantViolation::ExpressionDimension);
    };
    let column = home_field_column(field, home)?;
    Ok(match time_grain {
        Some(grain) => Expression::DateTrunc {
            unit: grain_unit(*grain),
            operand: Box::new(column),
        },
        None => column,
    })
}

/// The `Scan` column of `field`, a field of `home`.
///
/// # Errors
///
/// [`InvariantViolation::ForeignField`] for a field outside `home`.
pub(crate) fn home_field_column(
    field: &FieldPath,
    home: &Dataset,
) -> Result<Expression, InvariantViolation> {
    if field.dataset != home.name || home.field(&field.field).is_none() {
        return Err(InvariantViolation::ForeignField);
    }
    column_reference(field.field.as_str())
}

/// The unit a time grain truncates to; E6 truncates in the query time zone (E6-5.2).
fn grain_unit(grain: Grain) -> DateUnit {
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
