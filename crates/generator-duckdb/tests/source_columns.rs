//! A `Scan` column that names a catalog column, read by a development-line dataset metric
//! (E0D-3), a field that reads a lower-case catalog column, and a `Scan` with no columns
//! (E6-4.3, E6-4.4, PLN-2.2, issue #264).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use support::{
    aggregate, assert_golden, big_integer, column, component, decimal, generate, named, project,
    query, scan_of,
};

use menura_representations::logical_plan::PlanNode;

/// The reserved prefix of a `Scan` column that names a catalog column.
const SOURCE_PREFIX: &str = "menura.source.";

fn source_column(name: &str) -> String {
    format!("{SOURCE_PREFIX}{name}")
}

/// `SUM` of the field `AMOUNT` and of the catalog column `Amount`, read as `source.amount`.
fn field_and_source_column() -> PlanNode {
    let source = source_column("AMOUNT");
    let scanned = scan_of(
        "ORDERS",
        vec![
            column("AMOUNT", decimal(10, 2), true),
            column(&source, decimal(10, 2), true),
        ],
    );
    let summed = aggregate(
        scanned,
        Vec::new(),
        vec![
            component("FIELD_TOTAL", "SUM(AMOUNT)", None),
            component("SOURCE_TOTAL", &format!("SUM(\"{source}\")"), None),
        ],
        vec![
            column("FIELD_TOTAL", decimal(38, 2), true),
            column("SOURCE_TOTAL", decimal(38, 2), true),
        ],
    );
    project(
        summed,
        vec![
            named("FIELD_TOTAL", "FIELD_TOTAL"),
            named("SOURCE_TOTAL", "SOURCE_TOTAL"),
        ],
        vec![
            column("FIELD_TOTAL", decimal(38, 2), true),
            column("SOURCE_TOTAL", decimal(38, 2), true),
        ],
        support::from_model("/semantic_model/0/datasets/0/metrics/0"),
    )
}

#[test]
fn e6_4_3_a_source_column_is_emitted_in_the_catalog_spelling() {
    let generated = generate(
        &field_and_source_column(),
        &query(&[], &["FIELD_TOTAL", "SOURCE_TOTAL"]),
    )
    .expect("generated SQL");
    assert_golden(
        &generated.sql,
        include_str!("golden/e6_4_3_source_column.sql"),
    );
}

#[test]
fn e6_4_4_an_empty_scan_still_counts_rows() {
    let counted = aggregate(
        scan_of("ORDERS", Vec::new()),
        Vec::new(),
        vec![component("ORDER_COUNT", "COUNT(*)", None)],
        vec![column("ORDER_COUNT", big_integer(), false)],
    );
    let plan = project(
        counted,
        vec![named("ORDER_COUNT", "ORDER_COUNT")],
        vec![column("ORDER_COUNT", big_integer(), false)],
        support::from_model("/semantic_model/0/metrics/0"),
    );
    let generated = generate(&plan, &query(&[], &["ORDER_COUNT"])).expect("generated SQL");
    assert_golden(&generated.sql, include_str!("golden/e6_4_4_empty_scan.sql"));
}

#[test]
fn pln_2_2_a_field_reads_a_lower_case_catalog_column_in_its_spelling() {
    let plan = scan_of("ORDERS", vec![column("UNITS", big_integer(), true)]);
    let generated = generate(&plan, &query(&["UNITS"], &[])).expect("generated SQL");
    assert!(
        generated.sql.contains(r#""quantity" AS "UNITS""#),
        "{}",
        generated.sql
    );
}
