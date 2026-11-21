//! Tests of the `SemiJoin` of policy propagation (issue #315): a row is kept only if at least
//! one related row exists along the path and every related row is visible (E4-3.4, E4-6.5a), a
//! NULL foreign key never matches (E6-4.14), and the steps of a `SemiJoin` come after those of
//! its input (E6-4.12).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_generator_duckdb::InvariantViolation;
use menura_representations::logical_plan::PlanNode;
use support::{assert_golden, generate_propagation, orders_scan, query, semi_join, step};

/// `ORDERS` kept when its customer is in the north.
fn one_step() -> PlanNode {
    semi_join(
        orders_scan(),
        vec![step("ORDERS_CUSTOMER", "ORDERS", "CUSTOMERS")],
        "REGION = 'north'",
    )
}

/// `ORDERS` kept when the country of its customer is open.
fn two_steps() -> PlanNode {
    semi_join(
        orders_scan(),
        vec![
            step("ORDERS_CUSTOMER", "ORDERS", "CUSTOMERS"),
            step("CUSTOMERS_COUNTRY", "CUSTOMERS", "COUNTRIES"),
        ],
        "IS_OPEN",
    )
}

#[cfg(test)]
fn sql_of(plan: &PlanNode) -> String {
    generate_propagation(plan, &query(&["ORDER_ID", "CUSTOMER_ID"], &[]))
        .expect("SQL")
        .sql
}

#[test]
fn e4_3_4_semi_join_of_one_step() {
    assert_golden(
        &sql_of(&one_step()),
        include_str!("golden/e4_3_4_semi_join_one_step.sql"),
    );
}

#[test]
fn e4_6_5a_semi_join_of_a_two_step_chain() {
    assert_golden(
        &sql_of(&two_steps()),
        include_str!("golden/e4_6_5a_semi_join_two_steps.sql"),
    );
}

#[test]
fn e6_4_14_semi_join_null_key_never_matches() {
    // The keys are compared with `=`, so a NULL foreign key finds no related row and the row
    // is removed.
    let sql = sql_of(&one_step());
    assert_golden(&sql, include_str!("golden/e4_3_4_semi_join_one_step.sql"));
    assert!(!sql.contains("IS NOT DISTINCT FROM"), "{sql}");
    assert!(!sql.contains("COALESCE"), "{sql}");
}

#[test]
fn e4_6_5a_semi_join_non_unique_key_with_mixed_visibility() {
    // Every related row must be visible: the related rows are counted, and the row is kept
    // only when all of them pass, not when one does.
    let sql = sql_of(&two_steps());
    assert_golden(&sql, include_str!("golden/e4_6_5a_semi_join_two_steps.sql"));
    assert!(
        sql.contains(
            r#"WHERE "menura_related"."menura.rows" = "menura_related"."menura.visible_rows""#
        ),
        "{sql}"
    );
}

#[test]
fn e6_4_12_semi_join_steps_follow_its_input_then_its_path() {
    let sql = sql_of(&two_steps());
    let position = |name: &str| sql.find(name).expect(name);
    assert!(position(r#""menura_1_scan""#) < position(r#""menura_2_scan""#));
    assert!(position(r#""menura_2_scan""#) < position(r#""menura_3_scan""#));
    assert!(position(r#""menura_3_scan""#) < position(r#""menura_6_semijoin""#));
}

#[test]
fn e4_3_4_semi_join_without_path_is_an_error() {
    let plan = semi_join(orders_scan(), Vec::new(), "TRUE");
    let error = generate_propagation(&plan, &query(&["ORDER_ID", "CUSTOMER_ID"], &[]))
        .expect_err("no path");
    assert_eq!(error.violation, InvariantViolation::EmptyPath);
}

#[test]
fn e4_3_4_semi_join_along_an_unknown_relationship_is_an_error() {
    let plan = semi_join(
        orders_scan(),
        vec![step("ORDERS_SHOP", "ORDERS", "CUSTOMERS")],
        "TRUE",
    );
    let error = generate_propagation(&plan, &query(&["ORDER_ID", "CUSTOMER_ID"], &[]))
        .expect_err("unknown relationship");
    assert_eq!(error.violation, InvariantViolation::UnknownRelationship);
}

#[test]
fn e4_3_4_semi_join_step_that_does_not_follow_its_relationship_is_an_error() {
    let plan = semi_join(
        orders_scan(),
        vec![step("ORDERS_CUSTOMER", "ORDERS", "COUNTRIES")],
        "TRUE",
    );
    let error = generate_propagation(&plan, &query(&["ORDER_ID", "CUSTOMER_ID"], &[]))
        .expect_err("broken path");
    assert_eq!(error.violation, InvariantViolation::UnknownRelationship);
}
