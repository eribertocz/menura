//! E1-3.2 filter text, VS-2 SQL form and E2-5 rows 1 and 2: the checks after the JSON Schema.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_e1_parse::ParseErrorCode;
use serde_json::{Value, json};
use support::{
    accepted, assert_rejected, extension_data, rejected, wide_table_query, with_extension,
};

#[cfg(test)]
fn with_filter(filter: &str) -> Value {
    let mut document = wide_table_query();
    document["query"]["filter"] = json!(filter);
    document
}

#[cfg(test)]
fn sql_query(sql: &str) -> Value {
    json!({
        "contract": "menura.query",
        "contract_version": "1.1.0",
        "time_zone": "Europe/Madrid",
        "sql": sql
    })
}

#[test]
fn e1_3_2_filter_that_does_not_parse_is_e0f_01_at_the_filter() {
    assert_rejected(
        &with_filter("orders.region ="),
        ParseErrorCode::E0f01,
        "/query/filter",
    );
}

#[test]
fn e1_3_2_filter_outside_the_subset_is_e0f_02_at_the_filter() {
    assert_rejected(
        &with_filter("TRY_CAST(orders.amount AS INTEGER) > 1"),
        ParseErrorCode::E0f02,
        "/query/filter",
    );
}

#[test]
fn e1_3_2_bad_filters_item_after_a_valid_filter() {
    let mut document = with_extension(&extension_data(
        &json!({ "filters": ["orders.amount > 1", "orders.amount >"] }),
    ));
    document["query"]["filter"] = json!("orders.region = 'EU'");
    assert_rejected(
        &document,
        ParseErrorCode::E0f01,
        "/query/nonportable_extensions/0/data/filters/1",
    );
}

#[test]
fn e1_3_2_filter_is_checked_before_the_filters_items() {
    let mut document = with_extension(&extension_data(&json!({ "filters": ["RANDOM() > 1"] })));
    document["query"]["filter"] = json!("orders.amount >");
    assert_rejected(&document, ParseErrorCode::E0f01, "/query/filter");
}

#[test]
fn e1_3_2_filter_is_checked_after_the_json_schema() {
    let mut document = with_filter("orders.amount >");
    document["query"]["unknown"] = json!(true);
    assert_eq!(rejected(&document).code, ParseErrorCode::InvalidQuery);
}

#[test]
fn e1_3_2_filter_is_kept_as_a_parsed_expression() {
    let parsed = accepted(&with_filter("orders.region = 'EU'"));
    let filters = parsed.tree.filters();
    assert_eq!(filters.len(), 1);
    assert_eq!(
        filters.first().map(|filter| filter.origin.pointer()),
        Some("/query/filter")
    );
}

#[test]
fn vs_2_sql_form_is_not_yet_supported() {
    let error = rejected(&sql_query(
        "SELECT orders.region, AGG(revenue) FROM m.orders GROUP BY 1",
    ));
    assert_eq!(error.code, ParseErrorCode::NotYetSupported);
    assert!(error.message.contains("SQL form"), "{}", error.message);
    assert!(error.message.contains("0.5.0"), "{}", error.message);
}

#[test]
fn vs_2_sql_form_with_an_unsupported_construct_is_still_not_yet_supported() {
    let error = rejected(&sql_query(
        "WITH t AS (SELECT 1) SELECT * FROM t JOIN u ON TRUE",
    ));
    assert_eq!(error.code, ParseErrorCode::NotYetSupported);
}

#[test]
fn vs_2_sql_form_without_dimensions_or_metrics_is_not_yet_supported() {
    let error = rejected(&sql_query("SELECT 1"));
    assert_eq!(error.code, ParseErrorCode::NotYetSupported);
}

#[test]
fn vs_2_sql_form_is_detected_after_the_json_schema() {
    let mut document = sql_query("SELECT 1");
    document["unknown"] = json!(true);
    assert_eq!(rejected(&document).code, ParseErrorCode::InvalidQuery);
}

#[test]
fn qry_3_1_no_dimensions_and_no_metrics_is_empty_aggregation_query() {
    let mut document = wide_table_query();
    document["query"] = json!({ "dimensions": [], "metrics": [] });
    assert_eq!(
        rejected(&document).code,
        ParseErrorCode::EmptyAggregationQuery
    );
}

#[test]
fn qry_3_1_absent_dimensions_and_metrics_is_empty_aggregation_query() {
    let mut document = wide_table_query();
    document["query"] = json!({});
    assert_eq!(
        rejected(&document).code,
        ParseErrorCode::EmptyAggregationQuery
    );
}

#[test]
fn qry_3_1_filter_text_is_checked_before_the_empty_aggregation() {
    let mut document = wide_table_query();
    document["query"] = json!({ "filter": "orders.amount >" });
    assert_eq!(rejected(&document).code, ParseErrorCode::E0f01);
}

#[test]
fn qry_3_1_null_value_is_invalid_query() {
    let mut document = wide_table_query();
    document["query"]["filter"] = Value::Null;
    assert_rejected(&document, ParseErrorCode::InvalidQuery, "/query/filter");
}

#[test]
fn qry_3_1_one_metric_without_dimensions_is_accepted() {
    let mut document = wide_table_query();
    document["query"] = json!({ "metrics": ["revenue"] });
    assert_eq!(accepted(&document).tree.columns().metrics().count(), 1);
}
