//! Unit tests of the SQL of a `Snapshot` across a time `over` in the `ClickHouse` dialect: the
//! carried values keep their column names across the join that finds them (E6-7.1), and the
//! row that raises a runtime condition has the exact column types of the values it is united
//! with (E6-4.6b, E6-7.5, E6-7.8).

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
};
use support::{
    big_integer, column, component, decimal, from_model, generate_billing, named, parse,
    population_over, population_scan, query, validity,
};

const BALANCE_FIELDS: [&str; 4] = ["CUSTOMER_ID", "BALANCE_DATE", "BRANCH", "BALANCE"];
const CUSTOMER_FIELDS: [&str; 3] = ["CUSTOMER_ID", "VALID_FROM", "VALID_TO"];

/// The last `SUM(BALANCE)` of the global period per `BRANCH` and `CUSTOMER_ID`, across the
/// month of `BALANCE_DATE`, with `nulls` and `missing`.
fn snapshot_rule(nulls: NullHandling, missing: SnapshotMissing) -> SnapshotRule {
    let expected = population_over(
        population_scan("CUSTOMERS", &CUSTOMER_FIELDS),
        "CUSTOMERS",
        validity(PartialPeriod::AnyDay),
        None,
        (Grain::Month, population_scan("MONTHS", &["PERIOD"])),
    );
    SnapshotRule {
        metric: NormalizedName::new("BALANCE"),
        keys: vec![named("BRANCH", "BRANCH")],
        entity: vec![named("CUSTOMER_ID", "CUSTOMER_ID")],
        components: vec![component("BALANCE", "SUM(BALANCE)", None)],
        aggregation: SnapshotAggregation::Last(Selection::GlobalPeriod),
        nulls,
        over: SnapshotOver::Time(TimeSnapshot {
            period: parse("DATE_TRUNC('month', BALANCE_DATE)"),
            grain: Grain::Month,
            unit: None,
            expected: Box::new(expected),
            missing,
            unexpected: UnexpectedHandling::Include,
        }),
    }
}

/// Missing values carried forward at most three periods, a stale value raising `RADD-04`.
fn carry_forward() -> SnapshotMissing {
    SnapshotMissing::CarryForward {
        carried: Box::new(population_scan("BALANCES", &BALANCE_FIELDS)),
        max_carry_forward: NonZeroU64::MIN.saturating_add(2),
        on_stale: StaleHandling::Error,
    }
}

#[cfg(test)]
fn sql_of(nulls: NullHandling, missing: SnapshotMissing) -> String {
    let plan = PlanNode::new(
        Operation::Snapshot {
            input: Box::new(population_scan("BALANCES", &BALANCE_FIELDS)),
            rule: snapshot_rule(nulls, missing),
        },
        OutputSchema::new(vec![
            column("BRANCH", LogicalType::Text, true),
            column("CUSTOMER_ID", big_integer(), true),
            column("BALANCE", decimal(38, 2), true),
        ]),
        from_model("/semantic_model/0/metrics/0"),
    );
    generate_billing(&plan, &query(&["BRANCH", "CUSTOMER_ID"], &["BALANCE"]))
        .expect("SQL")
        .sql
}

/// The lines of the step `name` of `sql`, from its `SELECT` to its closing parenthesis.
fn step<'sql>(sql: &'sql str, name: &str) -> Vec<&'sql str> {
    let opening = format!("_{name}\" AS (");
    sql.lines()
        .skip_while(|line| !line.ends_with(&opening))
        .skip(1)
        .take_while(|line| !line.starts_with("  )"))
        .map(str::trim)
        .collect()
}

#[test]
fn e6_7_1_carried_values_keep_their_column_names() {
    let sql = sql_of(NullHandling::Keep, carry_forward());
    let carry = step(&sql, "carry");
    assert_eq!(
        carry[..4],
        [
            "SELECT",
            "\"menura_fill\".\"BRANCH\" AS \"BRANCH\",",
            "\"menura_fill\".\"CUSTOMER_ID\" AS \"CUSTOMER_ID\",",
            "\"menura_missing\".\"menura.period\" AS \"menura.period\",",
        ],
        "{carry:#?}"
    );
}

#[test]
fn e6_7_5_the_raising_row_has_the_column_types_of_the_values() {
    let sql = sql_of(NullHandling::Error, SnapshotMissing::Exclude);
    let snapshot = step(&sql, "snapshot");
    let raising: Vec<&str> = snapshot
        .iter()
        .skip_while(|line| **line != "UNION ALL")
        .copied()
        .collect();
    assert_eq!(raising[1], "SELECT", "{snapshot:#?}");
    assert_eq!(raising[2], "CAST(NULL AS Nullable(String)) AS \"BRANCH\",");
    assert_eq!(
        raising[3],
        "accurateCast(NULL, 'Nullable(Int64)') AS \"CUSTOMER_ID\","
    );
    assert!(
        raising[4].starts_with("CAST(nullIf(throwIf(")
            && raising[4].ends_with(", 0) AS Nullable(Decimal(38, 2))) AS \"BALANCE\""),
        "{}",
        raising[4]
    );
}

#[test]
fn e6_7_8_no_common_type_of_two_branches_is_a_variant() {
    let sql = sql_of(NullHandling::Error, SnapshotMissing::Exclude);
    assert!(sql.contains("\n  use_variant_as_common_type = 0"), "{sql}");
}
