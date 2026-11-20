//! The time dimension of context metrics in E0.e: CTX-5.1.4 and `E0C-61` (reachability).

use menura_contracts::admit_catalog_schema;
use menura_contracts::types::audit_record::OssieFormat;

use crate::extensions::context::fixture::{
    BILLED_YTD, CATALOG, MODEL, conformance_model, data, with,
};
use crate::extensions::summaries::SummaryDatasets;
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::references::context::resolve_context_references;
use crate::references::derived::resolve_derived_references;
use crate::references::resolve_references;
use crate::relationships::analyze_relationships;
use crate::relationships::context::{ResolvedMetrics, check_context_reachability};
use crate::structure::validate_structure;

/// The code and JSON Pointer of every `E0C-61` diagnostic of `model`, read in `format`.
fn problems_in(model: &str, format: OssieFormat) -> Vec<(&'static str, String)> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let catalog = admit_catalog_schema(CATALOG).expect("the catalog schema is admitted");
    let references =
        resolve_references(&document, &structure, &catalog, &SummaryDatasets::default())
            .expect("the references resolve");
    let derived = resolve_derived_references(&document, &structure, &extensions)
        .expect("the inputs of derived metrics resolve");
    let context =
        resolve_context_references(&document, &structure, &extensions).expect("the bases resolve");
    let relationships = analyze_relationships(
        &document,
        &structure.semantic_model,
        &extensions,
        &references,
    );
    let resolved = ResolvedMetrics {
        derived: &derived,
        context: &context,
    };
    match check_context_reachability(&structure, &references, resolved, &relationships) {
        Ok(()) => Vec::new(),
        Err(errors) => errors
            .iter()
            .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
            .collect(),
    }
}

fn problems(model: &str) -> Vec<(&'static str, String)> {
    problems_in(model, OssieFormat::Release)
}

fn billed_ytd() -> Vec<(&'static str, String)> {
    vec![("E0C-61", data(BILLED_YTD))]
}

/// [`MODEL`] with the `time_dimension` of `billed_ytd` replaced by `field`.
fn with_ytd_time_dimension(field: &str) -> String {
    with(
        r#""time_dimension": "invoices.invoice_date", "grain": "year"}}}'"#,
        &format!(r#""time_dimension": "{field}", "grain": "year"}}}}}}'"#),
    )
}

#[test]
fn ctx_5_1_4_the_examples_are_reachable() {
    assert!(problems(MODEL).is_empty());
    for model in ["e0-context-time-kind-001", "e0-context-cycle-001"] {
        assert!(problems(conformance_model(model)).is_empty(), "{model}");
    }
    assert!(
        problems_in(
            conformance_model("e0-context-development"),
            OssieFormat::Development
        )
        .is_empty()
    );
}

#[test]
fn ctx_5_1_4_e0c_61_a_time_dimension_not_reachable_from_the_base() {
    assert_eq!(
        problems(conformance_model("e0-context-unreachable-001")),
        billed_ytd()
    );
}

#[test]
fn ctx_5_1_4_e0c_61_a_time_dimension_that_does_not_exist() {
    for field in ["invoices.issued_on", "payments.paid_on"] {
        assert_eq!(
            problems(&with_ytd_time_dimension(field)),
            billed_ytd(),
            "{field}"
        );
    }
}

#[test]
fn ctx_5_1_4_a_field_of_a_reached_dataset_is_reachable() {
    assert!(problems(&with_ytd_time_dimension("customers.region")).is_empty());
}

#[test]
fn ctx_5_1_4_the_homes_of_a_derived_base_are_those_of_its_inputs() {
    let ytd = "'AGG(billed)'}]}\n        custom_extensions:\n          - vendor_name: COMMON\n            data: '{\"namespace\": \"menura\", \"version\": 1, \"context\": {\"base\": \"billed\", \"modifier\": {\"type\": \"period_to_date\"";
    let per_customer = with(
        "      - name: customer_count\n        expression: {dialects: [{dialect: ANSI_SQL, expression: 'COUNT(DISTINCT customers.customer_id)'}]}\n",
        "      - name: customer_count\n        expression: {dialects: [{dialect: ANSI_SQL, expression: 'COUNT(DISTINCT customers.customer_id)'}]}\n      \
         - name: billed_per_customer\n        expression: {dialects: [{dialect: ANSI_SQL, expression: 'AGG(billed) / AGG(customer_count)'}]}\n        \
         custom_extensions:\n          - vendor_name: COMMON\n            data: '{\"namespace\": \"menura\", \"version\": 1, \"derived\": {\"on_division_by_zero\": \"null\"}}'\n",
    );
    let model = per_customer.replacen(
        ytd,
        &ytd.replace("AGG(billed)", "AGG(billed_per_customer)")
            .replace("\"base\": \"billed\"", "\"base\": \"billed_per_customer\""),
        1,
    );
    assert_eq!(
        problems(&model),
        billed_ytd(),
        "customers does not reach invoices"
    );
}

#[test]
fn ctx_5_1_4_a_base_unusable_before_e0e_is_not_checked() {
    let model = conformance_model("e0-context-unusable").replacen(
        r#""type": "share_of_total", "remove": ["customers.region"]"#,
        r#""type": "period_to_date", "time_dimension": "payments.paid_on", "grain": "year""#,
        1,
    );
    assert!(problems(&model).is_empty());
}
