//! The `open_end` of a `validity` population that the plan or check reads (VAL-1.2, VAL-2.23,
//! E6-7.12): a value outside the range of the `ClickHouse` type of `valid_to` (`Date32` and
//! `DateTime64` cover 1900-01-01 to 2299-12-31, and only up to 2262-04-11 with precision `ns`)
//! cannot be written, so compilation fails with `E6-01` at `dataset:` of the dataset that
//! declares the population.

use menura_contracts::types::common::LogicalType;
use menura_model::internal_model::{InternalModel, NormalizedName, OpenEnd};
use menura_representations::logical_plan::{
    Membership, Operation, OutputSchema, PlanNode, PopulationRule,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::needs::{after_nanosecond_range, is_nanosecond_timestamp, outside_date32};
use crate::error::GenerateError;

/// `E6-01` at `dataset` when `open_end`, a value of `valid_to`, of `logical_type`, lies outside
/// the range of its `ClickHouse` type.
///
/// # Errors
///
/// [`GenerateError::InexpressibleType`] when it does.
pub(crate) fn check_open_end(
    open_end: Option<&OpenEnd>,
    logical_type: Option<&LogicalType>,
    dataset: &NormalizedName,
    model: &InternalModel,
) -> Result<(), GenerateError> {
    let Some(open_end) = open_end else {
        return Ok(());
    };
    let date = match open_end {
        OpenEnd::Date(date) => *date,
        OpenEnd::Timestamp(timestamp) => timestamp.date,
    };
    let nanoseconds = logical_type.is_some_and(is_nanosecond_timestamp);
    let outside = outside_date32(date) || (nanoseconds && after_nanosecond_range(date));
    match model.dataset(dataset) {
        Some(found) if outside => Err(GenerateError::InexpressibleType {
            origin: Origin::Model(found.location.clone()),
            subject: Some(format!("dataset:{}", dataset.as_str())),
        }),
        _ => Ok(()),
    }
}

/// `E6-01` for the `open_end` of the population `rule` reads over `rows`, declared by
/// `dataset`.
///
/// # Errors
///
/// As [`check_open_end`].
pub(crate) fn check_rule_open_end(
    rule: &PopulationRule,
    rows: &OutputSchema,
    dataset: &NormalizedName,
    model: &InternalModel,
) -> Result<(), GenerateError> {
    match &rule.membership {
        Membership::Validity {
            valid_to, open_end, ..
        } => check_open_end(
            open_end.as_ref(),
            column_type(rows, valid_to),
            dataset,
            model,
        ),
        Membership::Periodic { .. } => Ok(()),
    }
}

/// `E6-01` for the `open_end` that `node` reads: the coverage condition of a `Join` or
/// `SemiJoin` along a temporal relationship, located at its `to` dataset, or the rule of a
/// `Population`, located at the dataset that declares it.
///
/// # Errors
///
/// As [`check_open_end`].
pub(super) fn check_node_open_end(
    node: &PlanNode,
    model: &InternalModel,
) -> Result<(), GenerateError> {
    let (coverage, relationship, versions) = match node.operation() {
        Operation::Join {
            temporal: Some(coverage),
            relationship,
            right,
            ..
        } => (coverage, relationship, right.schema()),
        Operation::SemiJoin {
            temporal: Some(coverage),
            relationship,
            related_all,
            ..
        } => (coverage, relationship, related_all.schema()),
        Operation::Population {
            input,
            dataset,
            rule,
            ..
        } => return check_rule_open_end(rule, input.schema(), dataset, model),
        _ => return Ok(()),
    };
    let Some(to) = model.relationship(relationship).map(|found| &found.to) else {
        return Ok(());
    };
    check_open_end(
        coverage.range.open_end.as_ref(),
        column_type(versions, &coverage.range.valid_to),
        to,
        model,
    )
}

/// The type of the column `name` of `schema`, when it has one.
fn column_type<'schema>(
    schema: &'schema OutputSchema,
    name: &ColumnName,
) -> Option<&'schema LogicalType> {
    schema
        .columns()
        .iter()
        .find(|column| &column.name == name)
        .map(|column| &column.logical_type)
}
