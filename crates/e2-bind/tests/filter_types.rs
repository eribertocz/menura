//! E2-3.6 and the first check of E2-5 row 12b: every filter is typed as in E0.f.

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::BindCode;
use menura_representations::query_tree::QueryTreeParts;

use support::model::model;
use support::query::{dimension, metric, time_grain, tree};
use support::{bind_error, bind_query};

const ANALYST: &[&str] = &["analyst"];
const GRAINS: &str = "/query/nonportable_extensions/0/data/time_grains";

fn error(code: BindCode, pointer: &str) -> (BindCode, String) {
    (code, pointer.to_owned())
}

fn grain(
    index: usize,
    text: &str,
    grain: Grain,
) -> menura_representations::query_tree::TimeGrainChoice {
    time_grain(&format!("{GRAINS}/{index}"), text, grain)
}

/// The filter `expression` at `/query/filter`.
fn only_filter(expression: menura_model::expression::Expression) -> QueryTreeParts {
    QueryTreeParts {
        filters: vec![support::query::filter("/query/filter", expression)],
        ..QueryTreeParts::default()
    }
}

/// `orders.ordered_at + TRUE`: arithmetic on a timestamp and a boolean.
fn timestamp_plus_boolean() -> menura_model::expression::Expression {
    menura_model::expression::Expression::Binary {
        operator: menura_model::expression::BinaryOperator::Add,
        left: Box::new(support::query::column("orders.ordered_at")),
        right: Box::new(menura_model::expression::Expression::Literal(
            menura_model::expression::Literal::Boolean(true),
        )),
    }
}

fn typing(code: menura_expressions::typing::TypingCode) -> BindCode {
    BindCode::FilterType(code)
}

#[test]
fn e2_3_6_a_filter_that_is_not_boolean_is_e0f_06_at_the_filter() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        only_filter(support::query::column("orders.region")),
    );
    let diagnostic = support::bind_diagnostic(&query, &model(), ANALYST);
    assert_eq!(diagnostic.to_diagnostic().code, "E0F-06");
    assert_eq!(diagnostic.location.pointer(), "/query/filter");
}

#[test]
fn e2_3_6_a_typing_problem_in_a_filter_reports_its_e0f_code() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        only_filter(timestamp_plus_boolean()),
    );
    assert_eq!(
        bind_error(&query, &model(), ANALYST),
        error(
            typing(menura_expressions::typing::TypingCode::IncompatibleTypes),
            "/query/filter"
        )
    );
}

#[test]
fn e2_3_6_a_boolean_filter_is_accepted() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        only_filter(support::query::equals_text(support::query::column(
            "orders.region",
        ))),
    );
    assert!(bind_query(&query, &model(), ANALYST).is_ok());
}

#[test]
fn e2_5_rows_12a_and_12b_an_unusable_element_comes_before_a_filter_type() {
    let model = support::model::model_with(|parts| {
        support::model::mark_metric(parts, "REVENUE", &["E0D-04"]);
    });
    let mut parts = only_filter(support::query::column("orders.region"));
    parts.time_grains = Vec::new();
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        parts,
    );
    let diagnostic = support::bind_diagnostic(&query, &model, ANALYST);
    assert_eq!(diagnostic.to_diagnostic().code, "E0D-04");
    assert_eq!(diagnostic.location.pointer(), "/query/metrics/0");
}

#[test]
fn e2_5_rows_12b_a_filter_type_comes_before_e2_01() {
    let mut parts = only_filter(support::query::column("orders.region"));
    parts.time_grains = vec![grain(0, "orders.region", Grain::Month)];
    let query = tree(vec![dimension(0, "orders.region")], vec![], parts);
    let diagnostic = support::bind_diagnostic(&query, &model(), ANALYST);
    assert_eq!(diagnostic.to_diagnostic().code, "E0F-06");
}
