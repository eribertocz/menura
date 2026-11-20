//! E2-5 row 12c, E2-5.4, VS-1 and DER-2.2: a development-line model metric that reads
//! `AGG(dataset.metric)` is a derived metric, so it is outside the 0.3.0 slice even without a
//! `derived` extension; a context metric that reads its base with `AGG(…)` stays a context
//! metric. A metric also needs the capabilities of every metric it reads, recursively, and the
//! message names the one whose version comes first.

#[cfg(test)]
mod support;

use menura_e2_bind::{BindCode, BindDiagnostic};
use menura_model::internal_model::{
    ContextMetric, ContextModifier, DeclaredAdditivity, Metric, RemovedDimensions,
};
use menura_representations::query_tree::{QueryTree, QueryTreeParts};

use support::bind_diagnostic;
use support::model::{
    classification, customers_with_a_row_policy, model_with, name, semantic_metric,
};
use support::query::{dimension, metric, metric_aggregate, tree};

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
        modifier: ContextModifier::ShareOfTotal(RemovedDimensions::All),
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
fn der_2_2_a_metric_reading_a_dataset_metric_with_agg_is_a_derived_metric_of_0_4_0() {
    let model = model_with(|parts| {
        let mut read = classification(&["ORDERS"]);
        read.inputs = vec![name("ORDERS.ORDER_TOTAL")];
        semantic_metric(parts, "COST").classification = Some(read);
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.4.0", "derived");
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
            modifier: ContextModifier::ShareOfTotal(RemovedDimensions::All),
        });
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.8.0", "context");
}

#[test]
fn e2_5_4_a_context_metric_over_a_derived_base_names_derived_metrics_and_0_4_0() {
    let model = model_with(|parts| {
        context_over(semantic_metric(parts, "COST"), "revenue");
        read_through_agg(semantic_metric(parts, "REVENUE"), "orders.order_total");
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.4.0", "derived");
}

#[test]
fn e2_5_4a_a_derived_metric_over_an_additivity_declared_input_names_derived_metrics() {
    let model = model_with(|parts| {
        read_through_agg(semantic_metric(parts, "COST"), "revenue");
        semantic_metric(parts, "REVENUE").declared_additivity =
            Some(DeclaredAdditivity::NonAdditive);
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.4.0", "derived");
}

#[test]
fn e2_5_4_a_context_metric_over_an_additivity_declared_base_names_additivity_and_0_5_0() {
    let model = model_with(|parts| {
        context_over(semantic_metric(parts, "COST"), "revenue");
        semantic_metric(parts, "REVENUE").declared_additivity =
            Some(DeclaredAdditivity::NonAdditive);
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.5.0", "additivity");
}

#[test]
fn e2_5_4a_a_context_metric_whose_base_is_on_another_dataset_names_several_datasets() {
    let model = model_with(|parts| {
        context_over(semantic_metric(parts, "COST"), "revenue");
        semantic_metric(parts, "REVENUE").classification = Some(classification(&["CUSTOMERS"]));
    });
    let diagnostic = bind_diagnostic(&cost_by_region(), &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "0.4.0", "more than one dataset");
}

#[test]
fn vs_1_a_context_metric_whose_base_reaches_a_row_policy_names_propagation() {
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
    assert_not_yet_supported(&diagnostic, "0.4.0", "propagated");
}
