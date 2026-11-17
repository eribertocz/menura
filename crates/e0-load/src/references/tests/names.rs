//! Identifiers and uniqueness: E0D-1, E0D-2, `E0D-01`, DEV-2 and DEV-9, and malformed
//! delimited names (E0D-1.4).

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
    // A `model_name` that names both duplicates is `E0B-07` (E0B-3.2), so a third model is
    // selected: the duplicates are rejected even though neither is selected.
    let three = format!(
        "version: \"0.1.1\"\nsemantic_model:\n{}{}{}",
        model("shop"),
        model("SHOP"),
        model("other")
    );
    let rejection =
        resolve_selecting((&three, CATALOG), Some("other")).expect_err("the model is rejected");
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-01", "/semantic_model/1/name")]
    );
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

#[test]
fn e0d_1_4_e0d_01_a_malformed_delimited_dataset_name_is_rejected() {
    for files in [
        conformance_model!("e0-references-malformed-delimited-name-001"),
        conformance_model!("e0-references-malformed-delimited-name-002"),
        conformance_model!("e0-references-malformed-delimited-name-003"),
        conformance_model!("e0-references-malformed-delimited-name-004"),
    ] {
        let rejection = rejected(files);
        assert_eq!(
            summary(&rejection.errors),
            [pair("E0D-01", &format!("{ORDERS}/name"))]
        );
        assert!(rejection.errors[0].message.contains("malformed"));
        assert!(rejection.warnings.is_empty());
    }
}

#[test]
fn e0d_1_4_e0d_01_every_kind_of_declared_name_can_be_malformed() {
    let fields = document(&orders("order_id", &field("'\"a'", "amount")), "");
    assert_eq!(
        summary(&rejected((&fields, CATALOG)).errors),
        [pair("E0D-01", &format!("{ORDERS}/fields/2/name"))]
    );
    let metrics = document(
        &orders("order_id", ""),
        "    metrics: [{name: '\"\"', expression: {dialects: [{dialect: ANSI_SQL, expression: SUM(orders.amount)}]}}]\n",
    );
    assert_eq!(
        summary(&rejected((&metrics, CATALOG)).errors),
        [pair("E0D-01", "/semantic_model/0/metrics/0/name")]
    );
    let keys = document(&orders("'\"order_id'", ""), "");
    assert_eq!(
        summary(&rejected((&keys, CATALOG)).errors),
        [pair("E0D-01", &format!("{ORDERS}/primary_key/0"))]
    );
    let model = format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: '\"'\n    datasets: [{}]\n  - name: shop\n    datasets: [{}]\n",
        orders("order_id", ""),
        orders("order_id", "")
    );
    let rejection =
        resolve_selecting((&model, CATALOG), Some("shop")).expect_err("the model is rejected");
    assert_eq!(
        summary(&rejection.errors),
        [pair("E0D-01", "/semantic_model/0/name")]
    );
}

#[test]
fn e0d_1_4_e0d_01_a_malformed_relationship_column_is_rejected() {
    let relationships = document(
        &format!(
            "{}, {{name: customers, source: shop.main.customers, primary_key: [customer_id], fields: [\
             {{name: customer_id, expression: {{dialects: [{{dialect: ANSI_SQL, expression: customer_id}}]}}}}]}}",
            orders("order_id", &field("customer_id", "customer_id"))
        ),
        "    relationships: [{name: buyer, from: orders, to: customers, from_columns: ['\"a\"b\"'], to_columns: ['\"customer_id\"\"']}]\n",
    );
    let errors = rejected((&relationships, CATALOG)).errors;
    assert_eq!(
        summary(&errors),
        [
            pair("E0D-01", "/semantic_model/0/relationships/0/from_columns/0"),
            pair("E0D-01", "/semantic_model/0/relationships/0/to_columns/0"),
        ]
    );
}

#[test]
fn e0d_1_4_e0d_01_a_malformed_name_gives_one_diagnostic() {
    let long = format!("'\"{}'", "a".repeat(200));
    let repeated = document(
        &orders(
            "order_id",
            &format!("{}{}", field(&long, "amount"), field(&long, "amount")),
        ),
        "",
    );
    assert_eq!(
        summary(&rejected((&repeated, CATALOG)).errors),
        [
            pair("E0D-01", &format!("{ORDERS}/fields/2/name")),
            pair("E0D-01", &format!("{ORDERS}/fields/3/name")),
        ]
    );
}
