//! A `semi_additive` check (CHK-6.5) over `ORDERS` by `REGION` and `CREATED_AT`, with a time
//! `over` and the periodic population `population`.

use menura_contracts::types::common::Grain;
use menura_generator_clickhouse::{
    CheckInputs, GenerateError, GeneratedCheckSql, SnapshotCheck, SnapshotTime,
    generate_snapshot_check_sql,
};
use menura_model::internal_model::InternalModel;
use menura_representations::logical_plan::{Membership, PlanNode, PopulationRule};
use menura_representations::names::ColumnName;

use super::catalog_for;

/// The SQL of the monthly snapshot check of `rows`, whose population is `population`, by its
/// `REGION` and its `PERIOD`, over `model` and its catalog (`catalog_for`).
pub fn generate_snapshot(
    rows: &PlanNode,
    population: &PlanNode,
    model: &InternalModel,
) -> Result<GeneratedCheckSql, GenerateError> {
    let rule = PopulationRule {
        entity: vec![ColumnName::new("REGION")],
        membership: Membership::Periodic {
            period: ColumnName::new("CREATED_AT"),
        },
        required_when: None,
    };
    let check = SnapshotCheck {
        rows,
        entity: vec![ColumnName::new("REGION")],
        over: ColumnName::new("CREATED_AT"),
        time: Some(SnapshotTime {
            grain: Grain::Month,
            population,
            rule: &rule,
        }),
        result_columns: vec![
            "REGION".to_owned(),
            "CREATED_AT".to_owned(),
            "violation".to_owned(),
        ],
    };
    let catalog = catalog_for(model);
    generate_snapshot_check_sql(
        &check,
        CheckInputs {
            catalog: &catalog,
            model,
        },
    )
}
