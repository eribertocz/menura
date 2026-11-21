//! Tests of the `SemiJoin` of policy propagation (issue #315): a row is kept only if it has at
//! least one related row and every related row is visible (E4-3.4, E4-6.5a), a NULL key never
//! matches (E6-4.14), and the steps of a `SemiJoin` are those of its input, then of
//! `related_all`, then of `related_visible` (E6-4.12).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_generator_duckdb::{GenerateError, GeneratedSql, InvariantViolation};
use menura_representations::logical_plan::PlanNode;
use support::{
    assert_golden, big_integer, generate_propagation, orders_scan, policy, query, scan_fields,
    scan_typed, semi_join, semi_join_on,
};

/// The visible customers: those in the north.
fn north_customers(fields: &[&str]) -> PlanNode {
    policy(scan_fields("CUSTOMERS", fields), "REGION = 'north'")
}

/// `ORDERS` kept when its customer is in the north.
fn one_step() -> PlanNode {
    semi_join(
        orders_scan(),
        "ORDERS_CUSTOMER",
        "CUSTOMER_ID",
        scan_fields("CUSTOMERS", &["CUSTOMER_ID"]),
        north_customers(&["CUSTOMER_ID", "REGION"]),
    )
}

/// `ORDERS` kept when its customer is visible: in an open country.
fn two_steps() -> PlanNode {
    let visible_customers = semi_join(
        scan_fields("CUSTOMERS", &["CUSTOMER_ID", "COUNTRY_ID"]),
        "CUSTOMERS_COUNTRY",
        "COUNTRY_ID",
        scan_fields("COUNTRIES", &["COUNTRY_ID"]),
        policy(
            scan_fields("COUNTRIES", &["COUNTRY_ID", "IS_OPEN"]),
            "IS_OPEN",
        ),
    );
    semi_join(
        orders_scan(),
        "ORDERS_CUSTOMER",
        "CUSTOMER_ID",
        scan_fields("CUSTOMERS", &["CUSTOMER_ID"]),
        visible_customers,
    )
}

fn generate(plan: &PlanNode) -> Result<GeneratedSql, GenerateError> {
    generate_propagation(plan, &query(&["ORDER_ID", "CUSTOMER_ID"], &[]))
}

#[cfg(test)]
fn sql_of(plan: &PlanNode) -> String {
    generate(plan).expect("SQL").sql
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
    // The keys are compared with `=`, so a NULL key finds no related row and the row is
    // removed.
    let sql = sql_of(&one_step());
    assert_golden(&sql, include_str!("golden/e4_3_4_semi_join_one_step.sql"));
    assert!(
        sql.contains(r#"JOIN "menura_5_related" AS "menura_related" ON "menura_row"."CUSTOMER_ID" = "menura_related"."menura.key_1""#),
        "{sql}"
    );
    assert!(!sql.contains("IS NOT DISTINCT FROM"), "{sql}");
    assert!(!sql.contains("COALESCE"), "{sql}");
}

#[test]
fn e4_6_5a_semi_join_non_unique_key_with_mixed_visibility() {
    // Every related row must be visible: the related rows and the visible ones are counted per
    // key, and the row is kept only when the counts are equal, not when one row is visible.
    let sql = sql_of(&one_step());
    assert!(
        sql.contains(r#"WHERE "menura_related"."menura.rows" = "menura_visible"."menura.rows""#),
        "{sql}"
    );
    assert_eq!(
        sql.matches("COUNT(*) AS \"menura.rows\"").count(),
        2,
        "{sql}"
    );
}

#[test]
fn e6_4_12_semi_join_steps_follow_input_then_related_all_then_related_visible() {
    let sql = sql_of(&one_step());
    let position = |name: &str| sql.find(name).expect(name);
    assert!(position(r#""menura_1_scan" AS"#) < position(r#""menura_2_scan" AS"#));
    assert!(position(r#""menura_2_scan" AS"#) < position(r#""menura_3_scan" AS"#));
    assert!(position(r#""menura_3_scan" AS"#) < position(r#""menura_4_filter" AS"#));
    assert!(position(r#""menura_4_filter" AS"#) < position(r#""menura_7_semijoin" AS"#));
}

#[test]
fn e4_3_4_semi_join_without_key_is_an_error() {
    let plan = semi_join_on(
        orders_scan(),
        "ORDERS_CUSTOMER",
        Vec::new(),
        scan_fields("CUSTOMERS", &["CUSTOMER_ID"]),
        north_customers(&["CUSTOMER_ID", "REGION"]),
    );
    let error = generate(&plan).expect_err("no key");
    assert_eq!(error.violation, InvariantViolation::EmptyJoinCondition);
}

#[test]
fn e4_3_4_semi_join_on_an_unknown_column_is_an_error() {
    let plan = semi_join(
        orders_scan(),
        "ORDERS_CUSTOMER",
        "COUNTRY_ID",
        scan_fields("CUSTOMERS", &["CUSTOMER_ID"]),
        north_customers(&["CUSTOMER_ID", "REGION"]),
    );
    let error = generate(&plan).expect_err("unknown column");
    assert_eq!(error.violation, InvariantViolation::UnknownColumn);
}

#[test]
fn e4_3_4_semi_join_along_an_unknown_relationship_is_an_error() {
    let plan = semi_join(
        orders_scan(),
        "ORDERS_SHOP",
        "CUSTOMER_ID",
        scan_fields("CUSTOMERS", &["CUSTOMER_ID"]),
        north_customers(&["CUSTOMER_ID", "REGION"]),
    );
    let error = generate(&plan).expect_err("unknown relationship");
    assert_eq!(error.violation, InvariantViolation::UnknownRelationship);
}

/// `ORDERS` whose `CUSTOMER_ID` is typed `key_type`, kept when its customer is in the north.
fn typed_key(key_type: LogicalType) -> PlanNode {
    semi_join(
        scan_typed(
            "ORDERS",
            &[("ORDER_ID", big_integer()), ("CUSTOMER_ID", key_type)],
        ),
        "ORDERS_CUSTOMER",
        "CUSTOMER_ID",
        scan_fields("CUSTOMERS", &["CUSTOMER_ID"]),
        north_customers(&["CUSTOMER_ID", "REGION"]),
    )
}

#[test]
fn e6_4_13_semi_join_keys_of_two_types_are_cast_to_their_common_type() {
    let key_type = LogicalType::Integer {
        bits: IntegerBits::ThirtyTwo,
        signed: true,
    };
    let sql = sql_of(&typed_key(key_type));
    for alias in ["menura_related", "menura_visible"] {
        let condition = format!(
            r#"ON CAST("menura_row"."CUSTOMER_ID" AS HUGEINT) = CAST("{alias}"."menura.key_1" AS HUGEINT)"#
        );
        assert!(sql.contains(&condition), "{sql}");
    }
}

#[test]
fn e6_4_13_semi_join_keys_without_a_common_type_are_an_error() {
    let error = generate(&typed_key(LogicalType::Text)).expect_err("incompatible keys");
    assert_eq!(error.violation, InvariantViolation::IncompatibleJoinKeys);
}
