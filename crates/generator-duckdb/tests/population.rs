//! Tests of the `Population` node (issue #358): the expected (entity, period) pairs of a
//! population at a grain over the query's periods (E4-3.12, POP-3), with the SQL shape fixed
//! by golden files (E6-4.14) and the steps of its input before those of its periods (E6-4.12).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_generator_duckdb::InvariantViolation;
use menura_model::internal_model::PartialPeriod;
use menura_representations::logical_plan::{Membership, PlanNode};
use menura_representations::names::ColumnName;
use support::{assert_golden, generate_population, policy, population, population_scan, validity};

const CUSTOMER_FIELDS: [&str; 4] = ["CUSTOMER_ID", "VALID_FROM", "VALID_TO", "IS_BILLABLE"];

fn customers_population(partial_period: PartialPeriod, required_when: Option<&str>) -> PlanNode {
    population(
        population_scan("CUSTOMERS", &CUSTOMER_FIELDS),
        "CUSTOMERS",
        validity(partial_period),
        required_when,
        Grain::Month,
    )
}

#[cfg(test)]
fn sql_of(plan: &PlanNode) -> String {
    generate_population(plan).expect("SQL").sql
}

#[test]
fn e4_3_12_validity_any_day() {
    assert_golden(
        &sql_of(&customers_population(PartialPeriod::AnyDay, None)),
        include_str!("golden/e4_3_12_population_any_day.sql"),
    );
}

#[test]
fn e4_3_12_validity_full_period() {
    assert_golden(
        &sql_of(&customers_population(PartialPeriod::FullPeriod, None)),
        include_str!("golden/e4_3_12_population_full_period.sql"),
    );
}

#[test]
fn e4_3_12_validity_period_end_with_required_when() {
    assert_golden(
        &sql_of(&customers_population(
            PartialPeriod::PeriodEnd,
            Some("IS_BILLABLE"),
        )),
        include_str!("golden/e4_3_12_population_period_end_required.sql"),
    );
}

#[test]
fn e4_3_12_periodic() {
    let plan = population(
        population_scan("STATEMENTS", &["CUSTOMER_ID", "STATEMENT_MONTH"]),
        "STATEMENTS",
        Membership::Periodic {
            period: ColumnName::new("STATEMENT_MONTH"),
            grain: Grain::Month,
        },
        None,
        Grain::Month,
    );
    assert_golden(
        &sql_of(&plan),
        include_str!("golden/e4_3_12_population_periodic.sql"),
    );
}

#[test]
fn e6_4_12_population_reads_visible_rows_then_periods() {
    let visible = policy(
        population_scan("CUSTOMERS", &CUSTOMER_FIELDS),
        "IS_BILLABLE",
    );
    let plan = population(
        visible,
        "CUSTOMERS",
        validity(PartialPeriod::AnyDay),
        None,
        Grain::Quarter,
    );
    assert_golden(
        &sql_of(&plan),
        include_str!("golden/e6_4_12_population_visible_rows.sql"),
    );
}

#[test]
fn e4_3_12_an_entity_column_outside_the_input_is_an_invariant_violation() {
    let plan = population(
        population_scan("CUSTOMERS", &["VALID_FROM", "VALID_TO"]),
        "CUSTOMERS",
        validity(PartialPeriod::AnyDay),
        None,
        Grain::Month,
    );
    let error = generate_population(&plan).expect_err("an unknown column");
    assert_eq!(error.violation, InvariantViolation::UnknownColumn);
}
