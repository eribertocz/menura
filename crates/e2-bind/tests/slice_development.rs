//! E2-5 row 12c, E2-5.4, VS-1 and DER-2.2: a development-line model metric that reads
//! `AGG(dataset.metric)` is a derived metric, so it is outside the 0.3.0 slice even without a
//! `derived` extension; a context metric that reads its base with `AGG(…)` stays a context
//! metric.

#[cfg(test)]
mod support;

use menura_e2_bind::{BindCode, BindDiagnostic};
use menura_model::internal_model::{ContextMetric, ContextModifier, RemovedDimensions};
use menura_representations::query_tree::{QueryTree, QueryTreeParts};

use support::bind_diagnostic;
use support::model::{classification, model_with, name, semantic_metric};
use support::query::{dimension, metric, tree};

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
