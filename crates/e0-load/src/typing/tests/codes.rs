//! The typing codes `E0F-03` to `E0F-08`, `W0F-01`, CAT-4.3, E0F-4.1 and E0E-3.7.

use super::*;
use crate::typing::diagnostic::TypingCode;

#[test]
fn w0f_01_every_typing_problem_makes_its_element_unusable() {
    let typing = type_document(MODEL);
    assert_eq!(
        summary(&typing.warnings),
        [
            pair("E0F-04", &format!("{ORDERS}/fields/7/expression")),
            pair("E0F-03", &format!("{ORDERS}/fields/8/expression")),
            pair("E0F-07", &format!("{ORDERS}/fields/9/expression")),
            pair("E0F-06", &format!("{ORDERS}/fields/10/expression")),
            pair("E0F-05", "/semantic_model/0/relationships/1/from_columns/0"),
            pair("E0F-08", "/semantic_model/0/metrics/5/expression"),
            pair("E0F-07", "/semantic_model/0/metrics/6/expression"),
            pair("E0F-03", "/semantic_model/0/metrics/10/expression"),
            pair("E0F-03", "/semantic_model/0/metrics/11/expression"),
            pair("E0F-03", "/semantic_model/0/metrics/12/expression"),
            pair("E0F-03", "/semantic_model/0/metrics/13/expression"),
            pair("E0F-03", "/semantic_model/0/metrics/14/expression"),
        ]
    );
    assert_eq!(TypingCode::WARNING, "W0F-01");
    assert!(
        typing.warnings[0]
            .to_string()
            .starts_with("W0F-01 (E0F-04) at ")
    );
}

#[test]
fn e0_4_1_the_query_error_of_an_unusable_element_is_its_e0f_code() {
    let typing = type_document(MODEL);
    let unusable: Vec<(ElementKind, String, &str)> = typing
        .unusable
        .iter()
        .map(|element| {
            (
                element.kind,
                element.pointer.to_string(),
                element.query_error.as_str(),
            )
        })
        .collect();
    let element = |kind, pointer: &str, code| (kind, pointer.to_owned(), code);
    let field = |index: usize, code| {
        element(
            ElementKind::Field,
            &format!("{ORDERS}/fields/{index}"),
            code,
        )
    };
    let metric = |index: usize, code| {
        element(
            ElementKind::Metric,
            &format!("/semantic_model/0/metrics/{index}"),
            code,
        )
    };
    assert_eq!(
        unusable,
        [
            field(7, "E0F-04"),
            field(8, "E0F-03"),
            field(9, "E0F-07"),
            field(10, "E0F-06"),
            element(
                ElementKind::Relationship,
                "/semantic_model/0/relationships/1",
                "E0F-05"
            ),
            metric(5, "E0F-08"),
            metric(6, "E0F-07"),
            metric(10, "E0F-03"),
            metric(11, "E0F-03"),
            metric(12, "E0F-03"),
            metric(13, "E0F-03"),
            metric(14, "E0F-03"),
        ]
    );
}

#[test]
fn e0f_03_incompatible_types_without_cast() {
    for expression in [
        "status + 1",
        "status = 1",
        "ordered_on = ordered_at",
        "quantity || status",
        "quantity LIKE 'a%'",
        "CASE WHEN quantity > 1 THEN status ELSE quantity END",
        "COALESCE(status, quantity)",
        "GREATEST(price, quantity)",
        "LENGTH(quantity)",
        "LEFT(status, '1')",
        "YEAR(status)",
        "HOUR(ordered_on)",
        "DATE_TRUNC('hour', ordered_on)",
        "DATEADD(day, price, ordered_on)",
        "DATEDIFF(day, ordered_on, ordered_at)",
        "ROUND(price, quantity)",
        "ROUND(price, 3)",
        "ABS(status)",
    ] {
        assert_eq!(field(expression), Probe::Problem("E0F-03"), "{expression}");
    }
    assert_eq!(metric("SUM(orders.status)"), Probe::Problem("E0F-03"));
    assert_eq!(metric("AVG(orders.ordered_on)"), Probe::Problem("E0F-03"));
}

#[test]
fn e0f_04_cat_4_3_an_unsupported_column_cannot_be_used() {
    assert_eq!(field("payload"), Probe::Problem("E0F-04"));
    assert_eq!(field("payload IS NULL"), Probe::Problem("E0F-04"));
    assert_eq!(metric("COUNT(orders.payload)"), Probe::Untyped);
}

#[test]
fn e0f_05_e0f_4_1_e0e_3_7_relationship_key_pairs_have_one_kind() {
    let relationship = |from: &str, to: &str| {
        let model = MODEL.replace(
            "        from_columns: [customer_id]\n        to_columns: [customer_code]",
            &format!("        from_columns: [{from}]\n        to_columns: [{to}]"),
        );
        summary(&type_document(&model).warnings)
            .into_iter()
            .filter(|(code, _)| *code == "E0F-05")
            .count()
    };
    assert_eq!(relationship("customer_id", "customer_code"), 1);
    assert_eq!(
        relationship("customer_id", "customer_id"),
        0,
        "widths may differ"
    );
    assert_eq!(relationship("status", "customer_code"), 0);
    assert_eq!(relationship("status", "name"), 0);
    assert_eq!(
        relationship("order_id, status", "customer_id, customer_id"),
        1
    );
    assert_eq!(relationship("price", "customer_id"), 1);
    assert_eq!(relationship("discount", "customer_id"), 1);
    assert_eq!(
        relationship("missing", "customer_id"),
        0,
        "reported by E0.d"
    );
}

#[test]
fn e0f_4_1_decimal_keys_need_one_scale() {
    let model = MODEL
        .replace(
            "          - name: customer_code\n            expression: {dialects: [{dialect: ANSI_SQL, expression: customer_code}]}",
            "          - name: customer_code\n            expression: {dialects: [{dialect: ANSI_SQL, expression: 'CAST(customer_id AS DECIMAL(20, 2))'}]}\n          - name: customer_rate\n            expression: {dialects: [{dialect: ANSI_SQL, expression: 'CAST(customer_id AS DECIMAL(30, 20))'}]}",
        );
    let pairs = |to: &str| {
        let model = model.replace(
            "        from_columns: [customer_id]\n        to_columns: [customer_code]",
            &format!("        from_columns: [price]\n        to_columns: [{to}]"),
        );
        summary(&type_document(&model).warnings)
            .into_iter()
            .filter(|(code, _)| *code == "E0F-05")
            .count()
    };
    assert_eq!(pairs("customer_code"), 0, "decimal(10,2) and decimal(20,2)");
    assert_eq!(
        pairs("customer_rate"),
        1,
        "decimal(10,2) and decimal(30,20)"
    );
}

#[test]
fn e0f_06_an_expression_that_must_be_boolean_is_not() {
    for expression in [
        "CASE WHEN status THEN 1 ELSE 0 END",
        "NOT quantity",
        "quantity > 1 AND status",
        "IF(quantity, 1, 0)",
    ] {
        assert_eq!(field(expression), Probe::Problem("E0F-06"), "{expression}");
    }
    assert_eq!(
        metric("COUNT(*) FILTER (WHERE orders.quantity)"),
        Probe::Problem("E0F-06")
    );
}

#[test]
fn e0f_07_an_aggregate_where_none_is_allowed_or_nested() {
    assert_eq!(field("SUM(quantity)"), Probe::Problem("E0F-07"));
    assert_eq!(field("COUNT(*) + 1"), Probe::Problem("E0F-07"));
    assert_eq!(
        metric("SUM(COUNT(orders.order_id))"),
        Probe::Problem("E0F-07")
    );
    assert_eq!(
        metric("SUM(orders.quantity) FILTER (WHERE COUNT(*) > 1)"),
        Probe::Problem("E0F-07")
    );
}

#[test]
fn e0f_08_a_decimal_product_over_scale_38() {
    assert_eq!(field("rate * rate"), Probe::Problem("E0F-08"));
    assert_eq!(
        metric("SUM(orders.rate * orders.rate)"),
        Probe::Problem("E0F-08")
    );
    assert!(matches!(field("rate * price"), Probe::Typed(_)));
}
