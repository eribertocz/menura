//! The base and the removed fields of context metrics in E0.d: CTX-2.1, CTX-5.1.3, `E0C-60`,
//! `E0C-63` and `E0C-64`.

use semantic_compiler_contracts::types::audit_record::OssieFormat;

use crate::extensions::context::fixture::{
    BILLED, BILLED_PRIOR_YEAR, BILLED_SHARE, BILLED_YTD, MODEL, conformance_model, data, with,
};
use crate::extensions::{DEFAULT_NAMESPACE, ExtensionDiagnostic, validate_extensions};
use crate::read::read_document;
use crate::references::context::{ContextMetrics, resolve_context_references};
use crate::structure::validate_structure;

fn resolve_in(
    model: &str,
    format: OssieFormat,
) -> Result<ContextMetrics, Vec<ExtensionDiagnostic>> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    resolve_context_references(&document, &structure, &extensions)
}

fn rejected(model: &str) -> Vec<(&'static str, String)> {
    resolve_in(model, OssieFormat::Release)
        .expect_err("the model is rejected")
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

/// The pointer of each context metric and of its base.
fn bases(context: &ContextMetrics) -> Vec<(&str, &str)> {
    context
        .metrics
        .iter()
        .map(|metric| (metric.pointer.as_str(), metric.base.as_str()))
        .collect()
}

#[test]
fn ctx_2_1_ctx_5_1_3_the_bases_of_the_examples_resolve() {
    let context = resolve_in(MODEL, OssieFormat::Release).expect("the bases resolve");
    assert_eq!(
        bases(&context),
        [
            (BILLED_YTD, BILLED),
            (BILLED_PRIOR_YEAR, BILLED),
            (BILLED_SHARE, BILLED)
        ]
    );
    let metric = context.metric(&context.metrics[0].pointer).expect("found");
    assert_eq!(metric.name, "billed_ytd");
    assert_eq!(metric.diagnostic_pointer.to_string(), data(BILLED_YTD));
}

#[test]
fn ctx_5_1_3_e0c_60_a_base_that_names_no_metric() {
    assert_eq!(
        rejected(conformance_model("e0-context-base-001")),
        [pair("E0C-60", &data(BILLED_PRIOR_YEAR))]
    );
    let qualified = with(
        r#""base": "billed", "modifier": {"type": "share_of_total""#,
        r#""base": "invoices.billed", "modifier": {"type": "share_of_total""#,
    );
    assert_eq!(rejected(&qualified), [pair("E0C-60", &data(BILLED_SHARE))]);
}

#[test]
fn ctx_5_1_3_e0c_60_a_metric_with_both_context_and_derived() {
    assert_eq!(
        rejected(conformance_model("e0-context-derived-001")),
        [pair("E0C-60", &data(BILLED_YTD))]
    );
}

#[test]
fn ctx_5_1_3_e0c_60_an_expression_that_is_not_agg_of_the_base() {
    let ytd = "expression: 'AGG(billed)'}]}\n        custom_extensions:\n          - vendor_name: COMMON\n            data: '{\"namespace\": \"semantic_compiler\", \"version\": 1, \"context\": {\"base\": \"billed\", \"modifier\": {\"type\": \"period_to_date\"";
    for expression in [
        "AGG(billed) * 2",
        "AGG(customer_count)",
        "SUM(invoices.billed)",
        "AGG(billed) + AGG(billed)",
    ] {
        let model = with(ytd, &ytd.replace("AGG(billed)", expression));
        assert_eq!(
            rejected(&model),
            [pair("E0C-60", &data(BILLED_YTD))],
            "{expression}"
        );
    }
}

#[test]
fn ctx_5_1_3_the_base_is_compared_under_e0d_1_1_and_e0d_1_3() {
    let model = with(
        r#""base": "billed", "modifier": {"type": "share_of_total""#,
        r#""base": "BILLED", "modifier": {"type": "share_of_total""#,
    );
    let context = resolve_in(&model, OssieFormat::Release).expect("the bases resolve");
    assert_eq!(bases(&context)[2], (BILLED_SHARE, BILLED));
    let upper = with(
        r#""base": "billed", "modifier": {"type": "share_of_total""#,
        r#""base": "\"BILLED\"", "modifier": {"type": "share_of_total""#,
    );
    assert!(resolve_in(&upper, OssieFormat::Release).is_ok());
    let quoted = with(
        r#""base": "billed", "modifier": {"type": "share_of_total""#,
        r#""base": "\"billed\"", "modifier": {"type": "share_of_total""#,
    );
    assert_eq!(rejected(&quoted), [pair("E0C-60", &data(BILLED_SHARE))]);
}

#[test]
fn ctx_5_1_3_e0c_64_a_base_that_is_a_context_metric() {
    assert_eq!(
        rejected(conformance_model("e0-context-nested-001")),
        [pair("E0C-64", &data(BILLED_SHARE))]
    );
}

#[test]
fn ctx_3_7_ctx_5_1_3_a_derived_base_is_allowed() {
    let context = resolve_in(
        conformance_model("e0-context-cycle-001"),
        OssieFormat::Release,
    )
    .expect("the bases resolve");
    assert_eq!(
        bases(&context)[1],
        (BILLED_PRIOR_YEAR, "/semantic_model/0/metrics/4")
    );
}

#[test]
fn ctx_5_1_3_e0c_63_a_removed_field_that_does_not_exist() {
    assert_eq!(
        rejected(conformance_model("e0-context-removed-field-001")),
        [pair("E0C-63", &data(BILLED_SHARE))]
    );
    let dataset = with(
        r#""remove": ["customers.region"]"#,
        r#""remove": ["regions.region"]"#,
    );
    assert_eq!(rejected(&dataset), [pair("E0C-63", &data(BILLED_SHARE))]);
}

#[test]
fn ctx_5_1_3_the_development_format_resolves_a_derived_base() {
    let context = resolve_in(
        conformance_model("e0-context-development"),
        OssieFormat::Development,
    )
    .expect("the bases resolve");
    assert_eq!(
        bases(&context),
        [("/metrics/1", "/metrics/0"), ("/metrics/2", "/metrics/0")]
    );
    assert_eq!(
        context.metrics[0].diagnostic_pointer.to_string(),
        "/metrics/1/nonportable_extensions/0/data"
    );
}
