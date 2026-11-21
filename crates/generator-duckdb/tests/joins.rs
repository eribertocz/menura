//! Tests of the multi-node plan rules of issue #312: `Deduplicate` (E6-4.12, E6-4.14) and the
//! literals shared by several nodes (E6-4.2).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use support::{
    aggregate, assert_golden, big_integer, column, component, deduplicate, filter, from_query,
    generate, named, query, scan,
};

#[test]
fn e6_4_14_deduplicate_keeps_one_row_per_group_and_identity() {
    let deduplicated = deduplicate(
        scan("ORDERS", &["QUANTITY", "REGION", "ORDERED_ON"]),
        &["REGION", "ORDERED_ON"],
        &["QUANTITY"],
    );
    let plan = aggregate(
        deduplicated,
        vec![named("REGION", "REGION")],
        vec![component("ORDER_COUNT", "COUNT(*)", None)],
        vec![
            column("REGION", LogicalType::Text, true),
            column("ORDER_COUNT", big_integer(), false),
        ],
    );
    let generated = generate(&plan, &query(&["REGION"], &["ORDER_COUNT"])).expect("SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_14_deduplicate.sql"),
    );
}

#[test]
fn e6_4_14_deduplicate_without_group_partitions_by_identity() {
    let plan = deduplicate(scan("ORDERS", &["QUANTITY"]), &[], &["QUANTITY"]);
    let generated = generate(&plan, &query(&["QUANTITY"], &[])).expect("SQL");
    assert!(
        generated
            .sql
            .contains(r#"QUALIFY ROW_NUMBER() OVER (PARTITION BY "QUANTITY") = 1"#),
        "{}",
        generated.sql
    );
}

#[test]
fn e6_4_14_deduplicate_of_an_unknown_column_is_an_error() {
    let plan = deduplicate(scan("ORDERS", &["QUANTITY"]), &["REGION"], &["QUANTITY"]);
    assert!(generate(&plan, &query(&["QUANTITY"], &[])).is_err());
}

#[test]
fn e6_4_2_a_shared_literal_gives_one_parameter() {
    let once = filter(
        scan("ORDERS", &["REGION"]),
        "REGION <> 'west'",
        from_query("/query/filters/0"),
    );
    let twice = filter(once, "REGION <> 'west'", from_query("/query/filters/0"));
    let generated = generate(&twice, &query(&["REGION"], &[])).expect("SQL");
    assert_eq!(generated.parameters.len(), 1);
    assert_eq!(
        generated
            .sql
            .matches(r#""REGION" <> CAST($1 AS VARCHAR)"#)
            .count(),
        2,
        "{}",
        generated.sql
    );
}

#[test]
fn e6_4_2_each_literal_occurrence_is_its_own_parameter() {
    let plan = filter(
        filter(
            scan("ORDERS", &["REGION"]),
            "REGION <> 'west'",
            from_query("/query/filters/0"),
        ),
        "REGION <> 'west' AND REGION <> 'west'",
        from_query("/query/filters/1"),
    );
    let generated = generate(&plan, &query(&["REGION"], &[])).expect("SQL");
    assert_eq!(generated.parameters.len(), 3, "{}", generated.sql);
}

#[test]
fn e6_4_14_deduplicate_without_identity_is_an_error() {
    let plan = deduplicate(scan("ORDERS", &["REGION"]), &["REGION"], &[]);
    let error = generate(&plan, &query(&["REGION"], &[])).expect_err("no identity");
    assert_eq!(
        error.violation,
        menura_generator_duckdb::InvariantViolation::EmptyIdentity
    );
}
