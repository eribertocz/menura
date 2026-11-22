//! Tests of the `Periods` node (issue #358): one row per period start at a grain, from the
//! period of `from` to the period of `to`, both included (E4-3.14), the bounds bound as
//! parameters (E6-4.2, E6-4.11) and the SQL shape fixed by golden files (E6-4.14).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::DateValue;
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode};
use support::{assert_golden, column, from_query, generate_billing, instant, query};

#[cfg(test)]
fn periods(grain: Grain, logical_type: LogicalType) -> PlanNode {
    PlanNode::new(
        Operation::Periods {
            from: DateValue::new(2026, 2, 14).expect("a date"),
            to: DateValue::new(2026, 6, 30).expect("a date"),
            grain,
        },
        OutputSchema::new(vec![column("PERIOD", logical_type, false)]),
        from_query("/query/date_range"),
    )
}

#[test]
fn e4_3_14_month_periods_of_a_date_range() {
    let generated = generate_billing(
        &periods(Grain::Month, LogicalType::Date),
        &query(&["PERIOD"], &[]),
    )
    .expect("SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e4_3_14_periods_month.sql"),
    );
    assert_eq!(generated.parameters.len(), 2, "the two bounds");
}

#[test]
fn e4_3_14_week_periods_as_instants_in_the_query_time_zone() {
    let generated =
        generate_billing(&periods(Grain::Week, instant()), &query(&["PERIOD"], &[])).expect("SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e4_3_14_periods_week_instant.sql"),
    );
    assert_eq!(
        generated.parameters[0].logical_type,
        LogicalType::Text,
        "the time zone appears first (E6-4.2)"
    );
}
