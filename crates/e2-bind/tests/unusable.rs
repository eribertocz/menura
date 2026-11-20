//! E2-5 rows 10 to 12a: a query that names an element marked unusable in E0 fails with the
//! element's query error code, in the order of the table (E0-4.1, E0-4.2a).

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::BindCode;
use menura_model::internal_model::DiagnosticCode;
use menura_representations::query_tree::QueryTreeParts;

use support::bind_error;
use support::model::{mark_field, mark_metric, model_with};
use support::query::{dimension, metric, time_grain, tree};

const ANALYST: &[&str] = &["analyst"];

fn error(code: BindCode, pointer: &str) -> (BindCode, String) {
    (code, pointer.to_owned())
}

fn unusable(code: &str) -> BindCode {
    BindCode::Unusable(DiagnosticCode::new(code))
}

/// `orders.region` then the metrics `revenue` and `cost`.
fn two_metrics() -> menura_representations::query_tree::QueryTree {
    tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue"), metric(1, "cost")],
        QueryTreeParts::default(),
    )
}

#[test]
fn e2_5_row_10_a_metric_unusable_by_the_finer_grain_rule() {
    let model = model_with(|parts| {
        mark_metric(parts, "REVENUE", &["E_UNAGGREGATED_FINER_GRAIN_REFERENCE"]);
    });
    assert_eq!(
        bind_error(&two_metrics(), &model, &["admin"]),
        error(
            BindCode::UnaggregatedFinerGrainReference,
            "/query/metrics/0"
        )
    );
}

#[test]
fn e2_5_rows_10_and_11_row_10_comes_first_even_later_in_query_order() {
    let model = model_with(|parts| {
        mark_metric(parts, "REVENUE", &["E_DEFERRED_WINDOWED_METRIC"]);
        mark_metric(parts, "COST", &["E_UNAGGREGATED_FINER_GRAIN_REFERENCE"]);
    });
    assert_eq!(
        bind_error(&two_metrics(), &model, &["admin"]),
        error(
            BindCode::UnaggregatedFinerGrainReference,
            "/query/metrics/1"
        )
    );
}

#[test]
fn e2_5_rows_11_and_12_row_11_comes_first_even_later_in_query_order() {
    let model = model_with(|parts| {
        mark_metric(parts, "REVENUE", &["E_AMBIGUOUS_MEASURE_GRAIN"]);
        mark_metric(parts, "COST", &["E_DEFERRED_WINDOWED_METRIC"]);
    });
    assert_eq!(
        bind_error(&two_metrics(), &model, &["admin"]),
        error(BindCode::DeferredWindowedMetric, "/query/metrics/1")
    );
}

#[test]
fn e2_5_rows_12_and_12a_row_12_comes_first_even_later_in_query_order() {
    let model = model_with(|parts| {
        mark_field(parts, "ORDERS", "REGION", &["E0D-03"]);
        mark_metric(parts, "REVENUE", &["E_AMBIGUOUS_MEASURE_GRAIN"]);
    });
    assert_eq!(
        bind_error(&two_metrics(), &model, &["admin"]),
        error(BindCode::AmbiguousMeasureGrain, "/query/metrics/0")
    );
}

#[test]
fn e2_5_row_12a_an_unusable_field_reports_its_query_error_code() {
    let model = model_with(|parts| mark_field(parts, "ORDERS", "REGION", &["E0F-04"]));
    assert_eq!(
        bind_error(&two_metrics(), &model, &["admin"]),
        error(unusable("E0F-04"), "/query/dimensions/0")
    );
}

#[test]
fn e2_5_row_12a_an_element_with_several_codes_reports_the_first_in_the_row_order() {
    let cases: [(&[&str], &str); 6] = [
        (&["E0B-09", "E0B-08"], "E0B-08"),
        (&["E0D-03", "E0B-09"], "E0B-09"),
        (&["E0F-03", "E0D-07"], "E0D-07"),
        (&["E0G-01", "E0F-08"], "E0F-08"),
        (&["E0G-02", "E0G-01"], "E0G-01"),
        (&["E0C-37", "E0G-02"], "E0G-02"),
    ];
    for (codes, first) in cases {
        let model = model_with(|parts| mark_metric(parts, "REVENUE", codes));
        assert_eq!(
            bind_error(&two_metrics(), &model, &["admin"]),
            error(unusable(first), "/query/metrics/0"),
            "codes {codes:?}"
        );
    }
}

#[test]
fn e2_5_row_12a_the_first_unusable_element_in_query_order_is_reported() {
    let model = model_with(|parts| {
        mark_metric(parts, "REVENUE", &["E0B-08"]);
        mark_metric(parts, "COST", &["E0C-37"]);
    });
    let query = tree(
        vec![],
        vec![metric(0, "cost"), metric(1, "revenue")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        bind_error(&query, &model, &["admin"]),
        error(unusable("E0C-37"), "/query/metrics/0")
    );
}

#[test]
fn e2_5_row_12a_a_field_of_an_unusable_dataset_is_unusable() {
    let model = model_with(|parts| {
        let orders = parts
            .datasets
            .iter_mut()
            .find(|dataset| dataset.name.as_str() == "ORDERS")
            .expect("ORDERS");
        orders.query_errors = vec![DiagnosticCode::new("E0B-08")];
    });
    assert_eq!(
        bind_error(&two_metrics(), &model, &["admin"]),
        error(unusable("E0B-08"), "/query/dimensions/0")
    );
}

#[test]
fn e2_5_rows_12a_and_12b_an_unusable_element_comes_before_e2_01() {
    let model = model_with(|parts| mark_metric(parts, "REVENUE", &["E0D-05"]));
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts {
            time_grains: vec![time_grain(
                "/query/nonportable_extensions/0/data/time_grains/0",
                "orders.region",
                Grain::Month,
            )],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        bind_error(&query, &model, ANALYST),
        error(unusable("E0D-05"), "/query/metrics/0")
    );
}

#[test]
fn e2_5_row_12a_the_message_names_the_element_as_written_and_its_code() {
    let model = model_with(|parts| mark_metric(parts, "REVENUE", &["E0D-05"]));
    let diagnostic = support::bind_diagnostic(&two_metrics(), &model, &["admin"]);
    assert!(
        diagnostic.message.contains("`revenue`"),
        "{}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.contains("E0D-05"),
        "{}",
        diagnostic.message
    );
    assert_eq!(diagnostic.to_diagnostic().code, "E0D-05");
}
