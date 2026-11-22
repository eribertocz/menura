//! The parts of a `Snapshot` rule read from the semi-additive declaration (ADD-3.2): the
//! entity columns, the `over` column and the result key that bounds each unit (ADD-4.7).

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::Expression;
use menura_model::internal_model::{Dataset, FieldPath, NormalizedName, SemiAdditive, ValueType};
use menura_representations::bound_query::{BoundDimensionKind, BoundQuery};
use menura_representations::logical_plan::{NamedExpression, UnitKey};
use menura_representations::names::ColumnName;

use crate::columns::{column_reference, dimension_column};
use crate::error::InvariantViolation;
use crate::snapshot_grain::OverGrain;

#[cfg(test)]
#[path = "rule_tests.rs"]
mod tests;

/// The column of the entity field at `position` in the `Snapshot` output, from 0.
pub(crate) fn entity_column(position: usize) -> ColumnName {
    ColumnName::new(&format!("entity_{}", position + 1))
}

/// The entity fields of `declaration`, each read from the home `Scan` of `home` and named by
/// its position (ADD-3.2).
///
/// # Errors
///
/// [`InvariantViolation::ForeignField`] for an entity field outside `home`.
pub(crate) fn entity_columns(
    declaration: &SemiAdditive,
    home: &Dataset,
) -> Result<Vec<NamedExpression>, InvariantViolation> {
    declaration
        .entity
        .iter()
        .enumerate()
        .map(|(position, field)| {
            Ok(NamedExpression {
                name: entity_column(position),
                expression: home_field(field, home)?,
                derived: None,
            })
        })
        .collect()
}

/// The `over` field of `declaration`, read from the home `Scan` of `home`.
///
/// # Errors
///
/// [`InvariantViolation::ForeignField`] for an `over` field outside `home`.
pub(crate) fn over_column(
    declaration: &SemiAdditive,
    home: &Dataset,
) -> Result<Expression, InvariantViolation> {
    home_field(&declaration.over, home)
}

/// The home fields the `Snapshot` reads besides the keys and components: `over`, then the
/// entity fields.
pub(crate) fn snapshot_fields(declaration: &SemiAdditive) -> Vec<NormalizedName> {
    std::iter::once(&declaration.over)
        .chain(&declaration.entity)
        .map(|field| field.field.clone())
        .collect()
}

/// The type of the `over` field of `declaration` in `home`.
///
/// # Errors
///
/// [`InvariantViolation::UntypedField`] for a field without a logical type.
pub(crate) fn over_type<'model>(
    declaration: &SemiAdditive,
    home: &'model Dataset,
) -> Result<&'model LogicalType, InvariantViolation> {
    let field = home
        .field(&declaration.over.field)
        .ok_or(InvariantViolation::ForeignField)?;
    match field.typed.as_ref().map(|typed| &typed.value_type) {
        Some(ValueType::Logical(logical_type)) => Ok(logical_type),
        _ => Err(InvariantViolation::UntypedField),
    }
}

/// The result key that bounds each unit of periods: the first dimension of `query` that reads
/// `over`, with its query grain, when it is not finer than `snapshot_grain` (ADD-4.7, E4-7.8).
/// `None` when the query does not group by `over`, which makes the whole range one unit
/// (ADD-4.10); a finer grain is `E4-03`, reported after the row policies (E2-5 row 19).
pub(crate) fn unit_key(
    query: &BoundQuery,
    over: &FieldPath,
    over_type: &LogicalType,
    snapshot_grain: Grain,
) -> Option<UnitKey> {
    query
        .columns()
        .dimensions()
        .enumerate()
        .find_map(|(position, dimension)| match &dimension.kind {
            BoundDimensionKind::Field {
                field, time_grain, ..
            } if field == over => Some((position, OverGrain::of(*time_grain, over_type)?)),
            BoundDimensionKind::Field { .. } | BoundDimensionKind::Expression { .. } => None,
        })
        .and_then(|(position, grain)| match grain {
            OverGrain::Grain(grain) if !OverGrain::Grain(grain).is_finer_than(snapshot_grain) => {
                Some(UnitKey {
                    column: dimension_column(position),
                    grain,
                })
            }
            OverGrain::Grain(_) | OverGrain::FinerThanDay => None,
        })
}

/// The reference to the home `Scan` column of `field`, which the `Scan` names after the
/// field's normalized name (E6-4.12).
fn home_field(field: &FieldPath, home: &Dataset) -> Result<Expression, InvariantViolation> {
    if field.dataset != home.name || home.field(&field.field).is_none() {
        return Err(InvariantViolation::ForeignField);
    }
    column_reference(field.field.as_str())
}
