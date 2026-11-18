//! The context extension in E0.c: CTX-2, CTX-2.1, CTX-4, CTX-5.1.1, CTX-5.1.2, `E0C-02`,
//! `E0C-03`, `E0C-05`, `E0C-36` and `E0C-62`.
//!
//! The model documents of the `e0-context*` conformance cases are read here at compile time
//! (see [`super::fixture`]), so the rules are checked against the files the conformance suite
//! uses.

use std::num::NonZeroU64;

use semantic_compiler_contracts::types::audit_record::OssieFormat;
use semantic_compiler_contracts::types::common::Grain;

use super::Modifier;
use super::fixture::{
    BILLED_PRIOR_YEAR, BILLED_SHARE, BILLED_YTD, MODEL, conformance_model, data, with,
    with_share_context,
};
use super::model::Removed;
use crate::extensions::output::{Extensions, ExtensionsFailure};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::names::Name;
use crate::read::read_document;
use crate::structure::validate_structure;

fn validate_in(model: &str, format: OssieFormat) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
}

fn validate(model: &str) -> Result<Extensions, ExtensionsFailure> {
    validate_in(model, OssieFormat::Release)
}

/// The code and JSON Pointer of every rejecting diagnostic of `model`.
fn rejected_in(model: &str, format: OssieFormat) -> Vec<(&'static str, String)> {
    validate_in(model, format)
        .expect_err("the extensions are rejected")
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn rejected(model: &str) -> Vec<(&'static str, String)> {
    rejected_in(model, OssieFormat::Release)
}

/// The codes that reject [`MODEL`] when the `context` of `billed_share` is `context`.
fn share_codes(context: &str) -> Vec<&'static str> {
    rejected(&with_share_context(context))
        .into_iter()
        .map(|(code, pointer)| {
            assert_eq!(pointer, data(BILLED_SHARE));
            code
        })
        .collect()
}

/// The codes that reject [`MODEL`] when the `modifier` of `billed_share` is `modifier`.
fn modifier_codes(modifier: &str) -> Vec<&'static str> {
    share_codes(&format!(r#"{{"base": "billed", "modifier": {modifier}}}"#))
}

fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

#[test]
fn ctx_4_ctx_5_1_1_the_three_examples_are_read() {
    let extensions = validate(MODEL).expect("the extensions are accepted");
    let found: Vec<(String, &str, &Modifier)> = extensions
        .context
        .iter()
        .map(|declaration| {
            assert_eq!(
                declaration.data.to_string(),
                data(declaration.metric.as_str())
            );
            (
                declaration.metric.to_string(),
                declaration.base.as_str(),
                &declaration.modifier,
            )
        })
        .collect();
    let metrics: Vec<&str> = found.iter().map(|(metric, ..)| metric.as_str()).collect();
    assert_eq!(metrics, [BILLED_YTD, BILLED_PRIOR_YEAR, BILLED_SHARE]);
    assert!(found.iter().all(|(_, base, _)| *base == "billed"));
    let Modifier::PeriodToDate(time) = found[0].2 else {
        panic!("billed_ytd is period_to_date");
    };
    assert_eq!(time.grain, Grain::Year);
    assert_eq!(time.time_dimension.dataset, Name::regular("invoices"));
    assert_eq!(time.time_dimension.field, Name::regular("invoice_date"));
    let Modifier::PriorPeriod { time, offset } = found[1].2 else {
        panic!("billed_prior_year is prior_period");
    };
    assert_eq!((time.grain, *offset), (Grain::Year, NonZeroU64::MIN));
    let Modifier::ShareOfTotal(Removed::Fields(fields)) = found[2].2 else {
        panic!("billed_share is share_of_total");
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].dataset, Name::regular("customers"));
    let pointer = extensions.context[0].metric.clone();
    assert!(extensions.context_declaration(&pointer).is_some());
    assert!(extensions.is_context_metric(&pointer));
}

#[test]
fn ctx_5_1_1_e0c_03_a_value_outside_its_set() {
    assert_eq!(
        rejected(conformance_model("e0-context-wrong-value-001")),
        [pair("E0C-03", &data(BILLED_YTD))]
    );
}

#[test]
fn ctx_5_1_1_e0c_02_a_key_that_the_declared_type_does_not_list() {
    assert_eq!(
        rejected(conformance_model("e0-context-unknown-key-001")),
        [pair("E0C-02", &data(BILLED_YTD))]
    );
}

#[test]
fn ctx_5_1_1_e0c_36_a_context_metric_with_additivity() {
    assert_eq!(
        rejected(conformance_model("e0-context-additivity-001")),
        [pair("E0C-36", &data(BILLED_SHARE))]
    );
}

#[test]
fn ctx_5_1_1_e0c_62_an_offset_that_is_not_positive() {
    assert_eq!(
        rejected(conformance_model("e0-context-offset-001")),
        [pair("E0C-62", &data(BILLED_PRIOR_YEAR))]
    );
}

#[test]
fn ctx_5_1_1_e0c_62_an_offset_that_is_not_an_integer() {
    let prior = |offset: &str| {
        modifier_codes(&format!(
            r#"{{"type": "prior_period", "time_dimension": "invoices.invoice_date", "grain": "month", "offset": {offset}}}"#
        ))
    };
    for offset in ["0", "-1", "1.5", "1.0", "-2.5"] {
        assert_eq!(prior(offset), ["E0C-62"], "{offset}");
    }
    for offset in [r#""1""#, "true", "null", "[1]"] {
        assert_eq!(prior(offset), ["E0C-03"], "{offset}");
    }
    assert!(validate(&with_share_context(
        r#"{"base": "billed", "modifier": {"type": "prior_period", "time_dimension": "invoices.invoice_date", "grain": "quarter", "offset": 4}}"#
    ))
    .is_ok());
}

#[test]
fn ctx_5_1_1_e0c_03_the_shape_of_context() {
    for context in [
        "[]",
        r#""billed""#,
        r#"{"modifier": {"type": "share_of_total", "remove": "all"}}"#,
        r#"{"base": "billed"}"#,
        r#"{"base": 1, "modifier": {"type": "share_of_total", "remove": "all"}}"#,
        r#"{"base": "billed", "modifier": "share_of_total"}"#,
    ] {
        assert_eq!(share_codes(context), ["E0C-03"], "{context}");
    }
    assert_eq!(
        share_codes(
            r#"{"base": "billed", "modifier": {"type": "share_of_total", "remove": "all"}, "grain": "day"}"#
        ),
        ["E0C-02"]
    );
}

#[test]
fn ctx_5_1_1_e0c_03_missing_keys_and_values_outside_their_sets() {
    for modifier in [
        r#"{"remove": "all"}"#,
        r#"{"type": "year_to_date", "remove": "all"}"#,
        r#"{"type": "period_to_date", "grain": "year"}"#,
        r#"{"type": "period_to_date", "time_dimension": "invoices.invoice_date"}"#,
        r#"{"type": "period_to_date", "time_dimension": "invoices.invoice_date", "grain": "YEAR"}"#,
        r#"{"type": "prior_period", "time_dimension": "invoices.invoice_date", "grain": "year"}"#,
        r#"{"type": "share_of_total"}"#,
        r#"{"type": "share_of_total", "remove": "everything"}"#,
        r#"{"type": "share_of_total", "remove": []}"#,
        r#"{"type": "share_of_total", "remove": {"field": "customers.region"}}"#,
    ] {
        assert_eq!(modifier_codes(modifier), ["E0C-03"], "{modifier}");
    }
}

#[test]
fn ctx_5_1_1_without_a_valid_type_only_keys_no_type_defines_are_unknown() {
    assert_eq!(
        modifier_codes(r#"{"type": "year_to_date", "grain": "year", "remove": "all"}"#),
        ["E0C-03"]
    );
    assert_eq!(
        modifier_codes(r#"{"type": "year_to_date", "window": 3}"#),
        ["E0C-02", "E0C-03"]
    );
    assert_eq!(
        modifier_codes(
            r#"{"type": "share_of_total", "remove": "all", "grain": "year", "window": 3}"#
        ),
        ["E0C-02"]
    );
}

#[test]
fn ctx_5_1_1_remove_is_a_list_without_repeats() {
    let remove = |remove: &str| {
        modifier_codes(&format!(
            r#"{{"type": "share_of_total", "remove": {remove}}}"#
        ))
    };
    for value in [
        r#"["customers.region", "customers.region"]"#,
        r#"["customers.region", "CUSTOMERS.REGION"]"#,
        r#"["customers.region", 1]"#,
    ] {
        assert_eq!(remove(value), ["E0C-03"], "{value}");
    }
    let all = with_share_context(
        r#"{"base": "billed", "modifier": {"type": "share_of_total", "remove": "all"}}"#,
    );
    let extensions = validate(&all).expect("the extensions are accepted");
    assert_eq!(
        extensions.context[2].modifier,
        Modifier::ShareOfTotal(Removed::All)
    );
}

#[test]
fn ctx_5_1_2_field_references_are_dataset_dot_field() {
    for field in [r#""region""#, r#""customers.region + 1""#, r#""a.b.c""#] {
        assert_eq!(
            modifier_codes(&format!(
                r#"{{"type": "share_of_total", "remove": [{field}]}}"#
            )),
            ["E0C-03"],
            "{field}"
        );
    }
    let time = with(
        r#""time_dimension": "invoices.invoice_date", "grain": "year"}}}'"#,
        r#""time_dimension": "invoice_date", "grain": "year"}}}'"#,
    );
    assert_eq!(rejected(&time), [pair("E0C-03", &data(BILLED_YTD))]);
    let quoted = with(
        r#""remove": ["customers.region"]"#,
        r#""remove": ["\"customers\".region", "customers.\"region\""]"#,
    );
    assert!(validate(&quoted).is_ok(), "the quoted names differ");
}

#[test]
fn ctx_5_1_1_e0c_05_context_on_a_development_dataset_metric() {
    assert_eq!(
        rejected_in(
            conformance_model("e0-context-development-dataset-metric-001"),
            OssieFormat::Development
        ),
        [pair(
            "E0C-05",
            "/datasets/0/metrics/0/nonportable_extensions/0/data"
        )]
    );
}

#[test]
fn ctx_5_1_1_the_development_format_is_read_as_the_release_format() {
    let extensions = validate_in(
        conformance_model("e0-context-development"),
        OssieFormat::Development,
    )
    .expect("the extensions are accepted");
    let modifiers: Vec<&Modifier> = extensions
        .context
        .iter()
        .map(|declaration| &declaration.modifier)
        .collect();
    assert!(matches!(modifiers[0], Modifier::PeriodToDate(_)));
    assert_eq!(*modifiers[1], Modifier::ShareOfTotal(Removed::All));
}
