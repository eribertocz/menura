//! E2-5 row 12c, E2-5.4, VS-1c and DER-2.2: a development-line model metric that reads
//! `AGG(dataset.metric)` is a derived metric, which is in the 0.5.0 slice, like additivity; a
//! context metric that reads its base with `AGG(…)` stays a context metric. A metric also needs
//! the capabilities of every metric it reads, recursively.

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::{BindCode, BindDiagnostic};
use menura_model::internal_model::{
    ContextMetric, ContextModifier, DeclaredAdditivity, FieldPath, Metric,
};
use menura_representations::query_tree::{QueryTree, QueryTreeParts};

use support::model::{
    classification, customers_with_a_row_policy, model_with, name, semantic_metric,
};
use support::query::{dimension, metric, metric_aggregate, tree};
use support::{bind_diagnostic, bind_query};

const ADMIN: &[&str] = &["admin"];

fn assert_not_yet_supported(diagnostic: &BindDiagnostic, version: &str, capability: &str) {
    assert_eq!(diagnostic.code, BindCode::NotYetSupported);
    assert_eq!(diagnostic.location.pointer(), "/query/metrics/0");
    assert!(
        diagnostic
            .message
            .ends_with(&format!("not supported before {version}")),
        "{}",
        diagnostic.message
    );
    assert!(
        diagnostic.message.contains(capability),
        "{}",
        diagnostic.message
    );
}

/// Makes `metric` read `input` with `AGG(…)`, as a development-line derived metric does.
fn read_through_agg(metric: &mut Metric, input: &str) {
    metric.expression = Some(metric_aggregate(input));
    let mut read = classification(&["ORDERS"]);
    let input_name = input.rsplit('.').next().unwrap_or(input);
    read.inputs = vec![name(&input_name.to_uppercase())];
    metric.classification = Some(read);
}

/// Makes `metric` a context metric over `base`.
fn context_over(metric: &mut Metric, base: &str) {
    read_through_agg(metric, base);
    metric.context = Some(ContextMetric {
        base: name(&base.to_uppercase()),
        modifier: period_to_date(),
    });
}

fn cost_by_region() -> QueryTree {
    tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "cost")],
        QueryTreeParts::default(),
    )
}

#[test]
fn vs_1b_a_metric_reading_a_dataset_metric_with_agg_is_in_the_slice() {
    let model = model_with(|parts| {
        let mut read = classification(&["ORDERS"]);
        read.inputs = vec![name("ORDERS.ORDER_TOTAL")];
        semantic_metric(parts, "COST").classification = Some(read);
    });
    let result = bind_query(&cost_by_region(), &model, ADMIN);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn e2_5_4a_a_context_metric_reading_its_base_is_a_context_metric_of_0_8_0() {
    let model = model_with(|parts| {
        let cost = semantic_metric(parts, "COST");
        let mut read = classification(&["ORDERS"]);
        read.inputs = vec![name("REVENUE")];
        cost.classification = Some(read);
        cost.context = Some(ContextMetric {
            base: name("REVENUE"),
            modifier: period_to_date(),
        });
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.8.0", "context");
}

#[test]
fn e2_5_4_a_context_metric_over_a_derived_base_names_context_metrics_and_0_8_0() {
    let model = model_with(|parts| {
        context_over(semantic_metric(parts, "COST"), "revenue");
        read_through_agg(semantic_metric(parts, "REVENUE"), "orders.order_total");
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.8.0", "context");
}

#[test]
fn vs_1c_a_derived_metric_over_an_additivity_declared_input_is_in_the_slice() {
    let model = model_with(|parts| {
        read_through_agg(semantic_metric(parts, "COST"), "revenue");
        semantic_metric(parts, "REVENUE").declared_additivity =
            Some(DeclaredAdditivity::NonAdditive);
    });
    let result = bind_query(&cost_by_region(), &model, ADMIN);
    assert!(result.is_ok(), "{:?}", result.err());
}

#[test]
fn e2_5_4a_a_context_metric_over_an_additivity_declared_base_names_context_metrics() {
    let model = model_with(|parts| {
        context_over(semantic_metric(parts, "COST"), "revenue");
        semantic_metric(parts, "REVENUE").declared_additivity =
            Some(DeclaredAdditivity::NonAdditive);
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.8.0", "context");
}

#[test]
fn e2_5_4a_a_context_metric_whose_base_is_on_another_dataset_names_context_metrics() {
    let model = model_with(|parts| {
        context_over(semantic_metric(parts, "COST"), "revenue");
        semantic_metric(parts, "REVENUE").classification = Some(classification(&["CUSTOMERS"]));
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.8.0", "context");
}

#[test]
fn vs_1b_a_context_metric_whose_base_reaches_a_row_policy_names_context_metrics() {
    let model = model_with(|parts| {
        let cost = semantic_metric(parts, "COST");
        context_over(cost, "revenue");
        if let Some(read) = cost.classification.as_mut() {
            read.home_datasets.clear();
        }
        customers_with_a_row_policy(parts);
    });
    let query = tree(vec![], vec![metric(0, "cost")], QueryTreeParts::default());
    let diagnostic = bind_diagnostic(&query, &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.8.0", "context");
}

#[test]
fn e2_5_4_metrics_that_read_each_other_are_visited_once_and_reported() {
    let model = model_with(|parts| {
        read_through_agg(semantic_metric(parts, "COST"), "revenue");
        context_over(semantic_metric(parts, "REVENUE"), "cost");
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.8.0", "context");
}

/// A `period_to_date` modifier, which stays outside the slice until its own issue.
fn period_to_date() -> ContextModifier {
    ContextModifier::PeriodToDate {
        time_dimension: FieldPath {
            dataset: name("ORDERS"),
            field: name("ORDER_DATE"),
        },
        grain: Grain::Year,
    }
}
