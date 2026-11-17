//! Identifiers and uniqueness: E0D-1, E0D-2, `E0D-01`, DEV-2 and DEV-9.

use super::*;

#[test]
fn e0d_1_1_e0d_1_3_dev_2_names_that_differ_only_in_case_collide() {
    let rejection = rejected(conformance_model!("e0-references-duplicate-name-001"));
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-01", "/semantic_model/0/datasets/1/name")]
    );
    assert_eq!(rejection.errors[0].code.load_code(), "E0D-01");
    assert!(rejection.warnings.is_empty());
}

#[test]
fn e0d_1_2_e0d_01_a_name_longer_than_128_characters() {
    let rejection = rejected(conformance_model!("e0-references-long-name-001"));
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-01", &format!("{ORDERS}/fields/4/name"))]
    );
}

#[test]
fn e0d_1_3_each_kind_of_element_has_its_own_scope() {
    let references = loaded(conformance_model!("e0-references-identifiers"));
    assert_eq!(
        summary(&references.warnings),
        [pair("E0D-04", "/semantic_model/0/metrics/2/expression")]
    );
    let fields = document(
        &format!(
            "{}, {}",
            orders("order_id", ""),
            orders("order_id", "").replace("{name: orders", "{name: others")
        ),
        "    metrics: [{name: amount, expression: {dialects: [{dialect: ANSI_SQL, expression: SUM(orders.amount)}]}}]\n",
    );
    assert_eq!(loaded((&fields, CATALOG)), References::default());
    let repeated = document(&orders("order_id", &field("Amount", "amount")), "");
    assert_eq!(
        summary(&rejected((&repeated, CATALOG)).errors),
        [pair("E0D-01", &format!("{ORDERS}/fields/2/name"))]
    );
}

#[test]
fn e0d_1_1_quoted_names_compare_exactly_and_unquoted_names_in_upper_case() {
    let references = loaded(conformance_model!("e0-references-identifiers"));
    assert_eq!(
        unusable(&references),
        [(
            ElementKind::Metric,
            "/semantic_model/0/metrics/2".to_owned(),
            "E0D-04"
        )]
    );
    assert_eq!(references.warnings[0].code.load_code(), "W0D-01");
    assert!(references.warnings[0].message.contains("`orders`.`amount`"));
}
