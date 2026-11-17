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

#[test]
fn e0d_1_4_a_declared_name_between_double_quotes_is_delimited() {
    let references = loaded(conformance_model!("e0-references-delimited-names"));
    assert_eq!(
        summary(&references.warnings),
        [pair("E0D-04", "/semantic_model/0/metrics/1/expression")]
    );
    let rejection = rejected(conformance_model!("e0-references-delimited-duplicate-001"));
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-01", "/semantic_model/0/datasets/1/name")]
    );
}

#[test]
fn e0d_1_2_the_quotes_of_a_delimited_name_do_not_count() {
    let name = format!("'\"{}\"'", "a".repeat(128));
    let fits = document(&orders("order_id", &field(&name, "amount")), "");
    assert_eq!(loaded((&fits, CATALOG)), References::default());
    let longer = format!("'\"{}\"'", "a".repeat(129));
    let too_long = document(&orders("order_id", &field(&longer, "amount")), "");
    assert_eq!(
        summary(&rejected((&too_long, CATALOG)).errors),
        [pair("E0D-01", &format!("{ORDERS}/fields/2/name"))]
    );
}

#[test]
fn e0d_2_semantic_model_names_are_unique_in_the_document() {
    let model = |name: &str| {
        format!(
            "  - name: {name}\n    datasets: [{}]\n",
            orders("order_id", "")
        )
    };
    let two = format!(
        "version: \"0.1.1\"\nsemantic_model:\n{}{}",
        model("shop"),
        model("SHOP")
    );
    for selected in ["shop", "SHOP"] {
        let rejection =
            resolve_selecting((&two, CATALOG), Some(selected)).expect_err("the model is rejected");
        assert_eq!(
            summary(&rejection.errors),
            [pair("E0D-01", "/semantic_model/1/name")]
        );
    }
    let distinct = format!(
        "version: \"0.1.1\"\nsemantic_model:\n{}{}",
        model("shop"),
        model(&"b".repeat(129))
    );
    let rejection =
        resolve_selecting((&distinct, CATALOG), Some("shop")).expect_err("the model is rejected");
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-01", "/semantic_model/1/name")]
    );
}

#[test]
fn e0d_1_5_catalog_columns_match_regular_references_normalized_and_delimited_ones_exactly() {
    let files = conformance_model!("e0-references-catalog-columns");
    let references = loaded(files);
    assert_eq!(
        summary(&references.warnings),
        [pair(
            "E0D-03",
            "/semantic_model/0/datasets/0/fields/3/expression"
        )]
    );
    assert!(references.warnings[0].message.contains("`amount`"));
    let payments = |keys: &str, fields: &str| {
        document(
            &format!(
                "{{name: payments, source: shop.main.payments, primary_key: [{keys}], fields: [\
                 {{name: payment_id, expression: {{dialects: [{{dialect: ANSI_SQL, expression: payment_id}}]}}}}{fields}]}}"
            ),
            "",
        )
    };
    let regular_and_exact = payments("amount, '\"Amount\"'", "");
    assert_eq!(loaded((&regular_and_exact, files.1)), References::default());
    let wrong_case = payments("'\"amount\"'", "");
    assert_eq!(
        summary(&loaded((&wrong_case, files.1)).warnings),
        [pair("E0D-06", &format!("{ORDERS}/primary_key/0"))]
    );
    let field_reads_the_column = payments("'\"Amount\"'", &field("'\"Amount\"'", "AMOUNT"));
    assert_eq!(
        loaded((&field_reads_the_column, files.1)),
        References::default()
    );
}
