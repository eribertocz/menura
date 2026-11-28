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

#[cfg(test)]
mod tests {
    use menura_contracts::Sha256Hash;
    use menura_contracts::types::common::LogicalType;
    use menura_model::expression::DateValue;
    use menura_model::internal_model::{
        Dataset, Descriptions, InternalModel, ModelParts, NormalizedName, OpenEnd, PartialPeriod,
        RelationshipGraph, SourceLocation, ValidToBound,
    };
    use menura_representations::logical_plan::{
        Membership, OutputSchema, PopulationRule, SchemaColumn,
    };
    use menura_representations::names::ColumnName;

    use super::check_rule_open_end;

    /// A model with the one dataset `MEMBERS`.
    fn test_model() -> InternalModel {
        let members = Dataset {
            name: NormalizedName::new("MEMBERS"),
            source: "shop.members".to_owned(),
            fields: Vec::new(),
            metrics: Vec::new(),
            primary_key: Vec::new(),
            unique_keys: Vec::new(),
            access: None,
            row_policies: Vec::new(),
            populations: Vec::new(),
            summary: false,
            descriptions: Descriptions::default(),
            query_errors: Vec::new(),
            location: SourceLocation {
                pointer: "/semantic_model/0/datasets/0".to_owned(),
                line: 1,
                column: 1,
            },
        };
        let parts = ModelParts {
            name: NormalizedName::new("SHOP"),
            descriptions: Descriptions::default(),
            datasets: vec![members],
            relationships: Vec::new(),
            graph: RelationshipGraph::default(),
            metrics: Vec::new(),
            security_context: None,
            summaries: Vec::new(),
            checks: Vec::new(),
            unusable: Vec::new(),
            warnings: Vec::new(),
        };
        InternalModel::new(parts, Sha256Hash::try_from("0".repeat(64)).expect("a hash"))
    }

    fn rule(year: u16) -> PopulationRule {
        PopulationRule {
            entity: vec![ColumnName::new("CUSTOMER_ID")],
            membership: Membership::Validity {
                valid_from: ColumnName::new("VALID_FROM"),
                valid_to: ColumnName::new("VALID_TO"),
                partial_period: PartialPeriod::AnyDay,
                valid_to_bound: ValidToBound::Inclusive,
                open_end: DateValue::new(year, 12, 31).map(OpenEnd::Date),
            },
            required_when: None,
        }
    }

    fn rows() -> OutputSchema {
        OutputSchema::new(vec![SchemaColumn {
            name: ColumnName::new("VALID_TO"),
            logical_type: LogicalType::Date,
            nullable: true,
        }])
    }

    #[test]
    fn e6_7_12_a_population_open_end_after_2299_is_e6_01_at_its_dataset() {
        let model = test_model();
        let dataset = &model.parts().datasets[0].name;
        assert!(check_rule_open_end(&rule(2099), &rows(), dataset, &model).is_ok());
        let error = check_rule_open_end(&rule(9999), &rows(), dataset, &model).unwrap_err();
        let diagnostic = error.to_diagnostic().expect("E6-01");
        assert_eq!(diagnostic.code, "E6-01");
        assert!(
            diagnostic.message.contains("`dataset:MEMBERS`"),
            "{diagnostic:?}"
        );
        let unknown = NormalizedName::new("NOT_A_DATASET");
        assert!(check_rule_open_end(&rule(9999), &rows(), &unknown, &model).is_ok());
    }
}
