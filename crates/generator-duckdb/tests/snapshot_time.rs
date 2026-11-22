//! Tests of the `Snapshot` node across a time `over` (issue #358): each `nulls`, `unexpected`,
//! `missing`, `on_stale` and `selection` branch (ADD-4, E4-7), the runtime errors of ADD-7
//! raised in SQL (E6-4.6b), and the SQL shape fixed by golden files (E6-4.14).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use std::num::NonZeroU64;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::internal_model::{
    NormalizedName, NullHandling, PartialPeriod, Selection, SnapshotAggregation, StaleHandling,
    UnexpectedHandling,
};
use menura_representations::logical_plan::{
    Operation, OutputSchema, PlanNode, SnapshotMissing, SnapshotOver, SnapshotRule, TimeSnapshot,
    UnitKey,
};
use menura_representations::names::ColumnName;
use support::{
    assert_golden, big_integer, column, component, decimal, from_model, generate_billing, named,
    parse, population_over, population_scan, query, validity,
};

const BALANCE_FIELDS: [&str; 4] = ["CUSTOMER_ID", "BALANCE_DATE", "BRANCH", "BALANCE"];

/// The fields of the balances `rule` reads: `BOOKED_AT` too when it is `over`.
fn balance_fields(rule: Rule) -> Vec<&'static str> {
    let mut fields = BALANCE_FIELDS.to_vec();
    if rule.over_instant {
        fields.push("BOOKED_AT");
    }
    fields
}
const CUSTOMER_FIELDS: [&str; 3] = ["CUSTOMER_ID", "VALID_FROM", "VALID_TO"];

/// What a missing entity does, with the rows it reads.
#[derive(Clone, Copy)]
enum Missing {
    Exclude,
    Zero,
    CarryForward(StaleHandling),
    Error,
}

/// One rule of `BALANCE`: `SUM(BALANCE)` per `BRANCH` (and `BALANCE_YEAR` when `by_year`) and
/// `CUSTOMER_ID`, across the month of `BALANCE_DATE`.
#[derive(Clone, Copy)]
struct Rule {
    aggregation: SnapshotAggregation,
    nulls: NullHandling,
    unexpected: UnexpectedHandling,
    missing: Missing,
    by_year: bool,
    /// Whether `over` is the `timestamp with time zone` field `BOOKED_AT`.
    over_instant: bool,
}

const LAST: SnapshotAggregation = SnapshotAggregation::Last(Selection::GlobalPeriod);

const BASE: Rule = Rule {
    aggregation: LAST,
    nulls: NullHandling::Keep,
    unexpected: UnexpectedHandling::Include,
    missing: Missing::Exclude,
    by_year: true,
    over_instant: false,
};

fn missing(rule: Rule) -> SnapshotMissing {
    match rule.missing {
        Missing::Exclude => SnapshotMissing::Exclude,
        Missing::Error => SnapshotMissing::Error,
        Missing::Zero => SnapshotMissing::Zero {
            added: Box::new(population_scan("ADDED_BALANCES", &balance_fields(rule))),
        },
        Missing::CarryForward(on_stale) => SnapshotMissing::CarryForward {
            carried: Box::new(population_scan("BALANCES", &balance_fields(rule))),
            max_carry_forward: NonZeroU64::MIN.saturating_add(2),
            on_stale,
        },
    }
}

fn snapshot_rule(rule: Rule) -> SnapshotRule {
    let mut keys = vec![named("BRANCH", "BRANCH")];
    if rule.by_year {
        keys.push(named("BALANCE_YEAR", "DATE_TRUNC('year', BALANCE_DATE)"));
    }
    let (over, periods) = if rule.over_instant {
        ("BOOKED_AT", "INSTANTS")
    } else {
        ("BALANCE_DATE", "MONTHS")
    };
    let expected = population_over(
        population_scan("CUSTOMERS", &CUSTOMER_FIELDS),
        "CUSTOMERS",
        validity(PartialPeriod::AnyDay),
        None,
        (Grain::Month, population_scan(periods, &["PERIOD"])),
    );
    SnapshotRule {
        metric: NormalizedName::new("BALANCE"),
        keys,
        entity: vec![named("CUSTOMER_ID", "CUSTOMER_ID")],
        components: vec![component("BALANCE", "SUM(BALANCE)", None)],
        aggregation: rule.aggregation,
        nulls: rule.nulls,
        over: SnapshotOver::Time(TimeSnapshot {
            period: parse(&format!("DATE_TRUNC('month', {over})")),
            grain: Grain::Month,
            unit: rule.by_year.then(|| UnitKey {
                column: ColumnName::new("BALANCE_YEAR"),
                grain: Grain::Year,
            }),
            expected: Box::new(expected),
            missing: missing(rule),
            unexpected: rule.unexpected,
        }),
    }
}

#[cfg(test)]
fn sql_of(rule: Rule) -> String {
    let mut columns = vec![column("BRANCH", LogicalType::Text, true)];
    let mut dimensions = vec!["BRANCH"];
    if rule.by_year {
        columns.push(column("BALANCE_YEAR", LogicalType::Date, true));
        dimensions.push("BALANCE_YEAR");
    }
    columns.push(column("CUSTOMER_ID", big_integer(), true));
    columns.push(column("BALANCE", decimal(38, 2), true));
    dimensions.push("CUSTOMER_ID");
    let plan = PlanNode::new(
        Operation::Snapshot {
            input: Box::new(population_scan("BALANCES", &balance_fields(rule))),
            rule: snapshot_rule(rule),
        },
        OutputSchema::new(columns),
        from_model("/semantic_model/0/metrics/0"),
    );
    generate_billing(&plan, &query(&dimensions, &["BALANCE"]))
        .expect("SQL")
        .sql
}

#[test]
fn add_4_8_last_of_the_global_period_per_result_group() {
    assert_golden(
        &sql_of(BASE),
        include_str!("golden/add_4_8_snapshot_last_global.sql"),
    );
}

#[test]
fn add_4_10_first_of_the_global_period_without_a_unit_key() {
    let rule = Rule {
        aggregation: SnapshotAggregation::First(Selection::GlobalPeriod),
        by_year: false,
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/add_4_10_snapshot_first_global_whole_range.sql"),
    );
}

#[test]
fn e4_7_2_average_across_the_periods() {
    let rule = Rule {
        aggregation: SnapshotAggregation::Average,
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/e4_7_2_snapshot_time_average.sql"),
    );
}

#[test]
fn e4_7_7_per_entity_missing_in_the_last_period_of_the_unit() {
    let rule = Rule {
        aggregation: SnapshotAggregation::Last(Selection::PerEntity),
        missing: Missing::Zero,
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/e4_7_7_snapshot_per_entity_zero.sql"),
    );
}

#[test]
fn e4_7_7_per_entity_first_without_a_unit_key() {
    let rule = Rule {
        aggregation: SnapshotAggregation::First(Selection::PerEntity),
        missing: Missing::Error,
        by_year: false,
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/e4_7_7_snapshot_per_entity_first_error.sql"),
    );
}

#[test]
fn add_4_5_nulls_zero() {
    let rule = Rule {
        nulls: NullHandling::Zero,
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/add_4_5_snapshot_nulls_zero.sql"),
    );
}

#[test]
fn add_4_5_nulls_as_missing() {
    let rule = Rule {
        nulls: NullHandling::AsMissing,
        missing: Missing::CarryForward(StaleHandling::Exclude),
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/add_4_5_snapshot_nulls_as_missing.sql"),
    );
}

#[test]
fn radd_03_nulls_error() {
    let rule = Rule {
        nulls: NullHandling::Error,
        ..BASE
    };
    let sql = sql_of(rule);
    assert_golden(
        &sql,
        include_str!("golden/radd_03_snapshot_nulls_error.sql"),
    );
    assert!(sql.contains("'RADD-03: nulls: error and a NULL value exists in metric BALANCE'"));
}

#[test]
fn add_4_4_unexpected_exclude() {
    let rule = Rule {
        unexpected: UnexpectedHandling::Exclude,
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/add_4_4_snapshot_unexpected_exclude.sql"),
    );
}

#[test]
fn radd_02_unexpected_error() {
    let rule = Rule {
        unexpected: UnexpectedHandling::Error,
        ..BASE
    };
    let sql = sql_of(rule);
    assert_golden(
        &sql,
        include_str!("golden/radd_02_snapshot_unexpected_error.sql"),
    );
    assert!(
        sql.contains("'RADD-02: unexpected: error and an unexpected row exists in metric BALANCE'")
    );
}

#[test]
fn add_4_3_missing_zero() {
    let rule = Rule {
        missing: Missing::Zero,
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/add_4_3_snapshot_missing_zero.sql"),
    );
}

#[test]
fn radd_01_missing_error() {
    let rule = Rule {
        missing: Missing::Error,
        ..BASE
    };
    let sql = sql_of(rule);
    assert_golden(
        &sql,
        include_str!("golden/radd_01_snapshot_missing_error.sql"),
    );
    assert!(
        sql.contains(
            "'RADD-01: missing: error and an expected entity has no row in metric BALANCE'"
        )
    );
}

#[test]
fn add_4_3_carry_forward_drops_stale_values() {
    let rule = Rule {
        missing: Missing::CarryForward(StaleHandling::Exclude),
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/add_4_3_snapshot_carry_forward_exclude.sql"),
    );
}

#[test]
fn radd_04_carry_forward_stale_error() {
    let rule = Rule {
        missing: Missing::CarryForward(StaleHandling::Error),
        ..BASE
    };
    let sql = sql_of(rule);
    assert_golden(
        &sql,
        include_str!("golden/radd_04_snapshot_stale_error.sql"),
    );
    assert!(
        sql.contains("'RADD-04: on_stale: error and a carried value is too old in metric BALANCE'")
    );
}

#[test]
fn e6_4_6b_the_first_condition_in_code_order_is_raised() {
    let rule = Rule {
        nulls: NullHandling::Error,
        unexpected: UnexpectedHandling::Error,
        missing: Missing::Error,
        ..BASE
    };
    let sql = sql_of(rule);
    assert_golden(
        &sql,
        include_str!("golden/e6_4_6b_snapshot_conditions_in_order.sql"),
    );
    let first = sql.find("'RADD-01:").expect("RADD-01");
    let second = sql.find("'RADD-02:").expect("RADD-02");
    let third = sql.find("'RADD-03:").expect("RADD-03");
    assert!(first < second && second < third);
    assert!(sql.contains("error(MIN(\"menura.violation\"))"));
}

#[test]
fn pop_3_5_a_timestamp_over_is_compared_by_day_in_the_query_time_zone() {
    let rule = Rule {
        aggregation: SnapshotAggregation::Last(Selection::PerEntity),
        missing: Missing::Error,
        unexpected: UnexpectedHandling::Exclude,
        by_year: false,
        over_instant: true,
        ..BASE
    };
    assert_golden(
        &sql_of(rule),
        include_str!("golden/pop_3_5_snapshot_timestamp_over.sql"),
    );
}

#[test]
fn e6_4_6b_carried_values_are_never_checked_for_nulls() {
    let rule = Rule {
        nulls: NullHandling::Error,
        unexpected: UnexpectedHandling::Error,
        missing: Missing::CarryForward(StaleHandling::Exclude),
        ..BASE
    };
    let sql = sql_of(rule);
    assert_golden(
        &sql,
        include_str!("golden/e6_4_6b_snapshot_carried_nulls_unchecked.sql"),
    );
    let carried = sql
        .split("_carried\" AS (")
        .nth(1)
        .and_then(|rest| rest.split("\n  ),").next())
        .expect("a carried step");
    assert!(
        !carried.contains("RADD-03"),
        "the carried read raises nothing"
    );
}

#[test]
fn pop_3_7_missing_error_ignores_population_rows_without_an_entity() {
    let rule = Rule {
        missing: Missing::Error,
        ..BASE
    };
    let sql = sql_of(rule);
    assert!(sql.contains("WHERE \"menura_row\".\"CUSTOMER_ID\" IS NOT NULL"));
}
