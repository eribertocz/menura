//! Unit tests of the SQL rules of the compiled plan contract (PLN-2).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use support::{filter, from_model, from_query, generate, limit, query, region_count, scan, sort};

#[test]
fn pln_2_1_query_and_security_values_never_appear_in_the_sql() {
    let plan = filter(
        filter(
            scan("ORDERS", &["REGION"]),
            "REGION IN (USER_ATTRIBUTE('regions'))",
            from_model("/semantic_model/0/datasets/0/row_policies/0"),
        ),
        "REGION IN ('east', 'west')",
        from_query("/query/filters/0"),
    );
    let generated = generate(&plan, &query(&["REGION"], &[])).expect("SQL");
    for value in ["north", "south", "east", "west"] {
        assert!(!generated.sql.contains(value), "{value} in the SQL");
    }
    assert!(
        generated
            .sql
            .contains(r#""REGION" IN (CAST($2 AS VARCHAR), CAST($3 AS VARCHAR))"#)
    );
}

#[test]
fn pln_2_2_identifiers_are_quoted_as_the_catalog_spells_them() {
    let plan = scan("ORDERS", &["AMOUNT", "REGION"]);
    let generated = generate(&plan, &query(&["AMOUNT", "REGION"], &[])).expect("SQL");
    assert!(generated.sql.contains(r#""Amount" AS "AMOUNT""#));
    assert!(generated.sql.contains(r#""Sales Region" AS "REGION""#));
    assert!(generated.sql.contains(r#"FROM "main"."orders""#));
}

#[test]
fn pln_2_3_the_statement_is_read_only() {
    let plan = sort(scan("ORDERS", &["REGION"]), &[("REGION", false, false)]);
    let generated = generate(&plan, &query(&["REGION"], &[])).expect("SQL");
    assert!(generated.sql.starts_with("WITH\n"));
    for word in [
        "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "COPY", "ATTACH", ";",
    ] {
        assert!(!generated.sql.contains(word), "{word} in the SQL");
    }
}

#[test]
fn pln_2_4_identical_inputs_give_identical_output() {
    let build = || {
        let filtered = filter(
            region_count(),
            "ORDER_COUNT > 1",
            from_query("/query/filters/0"),
        );
        limit(sort(filtered, &[("REGION", false, true)]), 3)
    };
    let columns = query(&["REGION"], &["ORDER_COUNT", "TOTAL_QUANTITY"]);
    let first = generate(&build(), &columns).expect("SQL");
    let second = generate(&build(), &columns).expect("SQL");
    assert_eq!(first, second);
    assert_eq!(
        format!("{:?}", first.parameters),
        format!("{:?}", second.parameters)
    );
}
