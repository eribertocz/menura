//! The context metrics in E0.f: CTX-3a.3, CTX-5.1.4 (`E0C-61`, time kind), CTX-5.1.5
//! (`E0C-34`) and the result types of CTX-5.1.7.

use menura_contracts::admit_catalog_schema;
use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::types::common::LogicalType;

use crate::extensions::context::fixture::{
    BILLED_CHANGE, BILLED_PRIOR_YEAR, BILLED_SHARE, BILLED_YTD, CATALOG, MODEL, conformance_model,
    data, with,
};
use crate::extensions::summaries::SummaryDatasets;
use crate::extensions::{DEFAULT_NAMESPACE, ExtensionDiagnostic, validate_extensions};
use crate::read::read_document;
use crate::references::context::resolve_context_references;
use crate::references::derived::resolve_derived_references;
use crate::structure::validate_structure;
use crate::typing::context::check_context_time_kinds;
use crate::typing::derived::type_derived_metrics;
use crate::typing::value_type::{Typed, double, huge_integer, integer};
use crate::typing::{Typing, type_expressions};

/// What E0.f finds in `model`, read in `format`: the typing of the derived and context
/// metrics or the diagnostics that reject the model, and the time-kind check.
fn type_in(
    model: &str,
    format: OssieFormat,
) -> (
    Result<Typing, Vec<ExtensionDiagnostic>>,
    Result<(), Vec<ExtensionDiagnostic>>,
) {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let catalog = admit_catalog_schema(CATALOG).expect("the catalog schema is admitted");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let derived =
        resolve_derived_references(&document, &structure, &extensions).expect("the inputs resolve");
    let context =
        resolve_context_references(&document, &structure, &extensions).expect("the bases resolve");
    let typing = type_expressions(
        &document,
        &structure,
        &catalog,
        &derived,
        &SummaryDatasets::default(),
    );
    let dependents =
        type_derived_metrics(&document, &structure, &catalog, &typing, &derived, &context);
    let time_kinds = check_context_time_kinds(&structure, &typing, &context);
    (dependents, time_kinds)
}

fn codes(found: Result<(), Vec<ExtensionDiagnostic>>) -> Vec<(&'static str, String)> {
    found
        .err()
        .unwrap_or_default()
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

/// The type of the metric at `pointer` in `typing`.
fn type_of(typing: &Typing, pointer: &str) -> Option<Typed> {
    typing
        .elements
        .iter()
        .find(|element| element.pointer.as_str() == pointer)
        .map(|element| element.typed.clone())
}

/// `SUM` of a 32-bit integer column is a 128-bit integer (E0F-5a).
fn sum_of_integer() -> LogicalType {
    huge_integer()
}

#[test]
fn ctx_5_1_7_the_result_type_is_the_base_type_and_nullable() {
    let (typing, time_kinds) = type_in(MODEL, OssieFormat::Release);
    let typing = typing.expect("the model is accepted");
    assert!(time_kinds.is_ok());
    assert!(typing.warnings.is_empty() && typing.unusable.is_empty());
    for metric in [BILLED_YTD, BILLED_PRIOR_YEAR] {
        assert_eq!(
            type_of(&typing, metric),
            Some(Typed::value(sum_of_integer(), true)),
            "{metric}"
        );
    }
    assert_ne!(sum_of_integer(), integer());
}

#[test]
fn ctx_3a_3_ctx_5_1_7_share_of_total_is_a_nullable_float() {
    let typing = type_in(MODEL, OssieFormat::Release)
        .0
        .expect("the model is accepted");
    assert_eq!(
        type_of(&typing, BILLED_SHARE),
        Some(Typed::value(double(), true))
    );
}

#[test]
fn ctx_3_7_a_derived_metric_reads_a_context_metric() {
    let typing = type_in(MODEL, OssieFormat::Release)
        .0
        .expect("the model is accepted");
    assert_eq!(
        type_of(&typing, BILLED_CHANGE),
        Some(Typed::value(sum_of_integer(), true)),
        "the difference of a non-null and a nullable 128-bit integer"
    );
}

#[test]
fn ctx_5_1_5_e0c_34_a_cycle_through_a_derived_and_a_context_metric() {
    let errors = type_in(
        conformance_model("e0-context-cycle-001"),
        OssieFormat::Release,
    )
    .0
    .expect_err("the model is rejected");
    let found: Vec<(&str, String)> = errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect();
    assert_eq!(found, [("E0C-34", data(BILLED_PRIOR_YEAR))]);
    assert!(
        errors[0]
            .message
            .ends_with("billed_prior_year → billed_change → billed_prior_year"),
        "{}",
        errors[0].message
    );
}

#[test]
fn ctx_5_1_4_e0c_61_a_time_dimension_that_is_not_a_time_field() {
    let (typing, time_kinds) = type_in(
        conformance_model("e0-context-time-kind-001"),
        OssieFormat::Release,
    );
    assert!(typing.is_ok());
    assert_eq!(codes(time_kinds), [("E0C-61", data(BILLED_YTD))]);
}

#[test]
fn ctx_5_1_4_an_untyped_time_dimension_is_not_checked() {
    let catalog_column = with(
        r#""time_dimension": "invoices.invoice_date", "grain": "year"}}}'"#,
        r#""time_dimension": "invoices.due_date", "grain": "year"}}}'"#,
    )
    .replacen(
        "          - name: billed\n            expression: {dialects: [{dialect: ANSI_SQL, expression: billed}]}\n",
        "          - name: billed\n            expression: {dialects: [{dialect: ANSI_SQL, expression: billed}]}\n          \
         - name: due_date\n            expression: {dialects: [{dialect: ANSI_SQL, expression: due_on}]}\n",
        1,
    );
    assert!(type_in(&catalog_column, OssieFormat::Release).1.is_ok());
}

#[test]
fn ctx_5_1_7_the_development_format_types_a_context_metric_over_a_derived_base() {
    let (typing, time_kinds) = type_in(
        conformance_model("e0-context-development"),
        OssieFormat::Development,
    );
    let typing = typing.expect("the model is accepted");
    assert!(time_kinds.is_ok());
    assert_eq!(
        type_of(&typing, "/metrics/1"),
        Some(Typed::value(sum_of_integer(), true))
    );
    assert_eq!(
        type_of(&typing, "/metrics/2"),
        Some(Typed::value(double(), true))
    );
}
