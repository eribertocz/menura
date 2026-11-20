//! E1-2.4: `WQRY-01` for the former contract name and the former extension namespace.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_e1_parse::ParseErrorCode;
use serde_json::json;
use support::{accepted, extension_data, rejected, wide_table_query, with_extension};

#[cfg(test)]
fn warning_pointers(document: &serde_json::Value) -> Vec<String> {
    accepted(document)
        .warnings
        .iter()
        .map(|warning| {
            assert_eq!(warning.code(), "WQRY-01");
            warning.pointer.clone()
        })
        .collect()
}

#[test]
fn e1_2_4_current_names_report_no_warning() {
    let document = with_extension(&extension_data(&json!({ "limit": 1 })));
    assert!(warning_pointers(&document).is_empty());
}

#[test]
fn e1_2_4_former_contract_name_is_wqry_01_at_contract() {
    let mut document = wide_table_query();
    document["contract"] = json!("semantic_compiler.query");
    assert_eq!(warning_pointers(&document), ["/contract"]);
}

#[test]
fn e1_2_4_former_namespace_is_wqry_01_at_the_namespace() {
    let mut data = extension_data(&json!({}));
    data["namespace"] = json!("semantic_compiler");
    assert_eq!(
        warning_pointers(&with_extension(&data)),
        ["/query/nonportable_extensions/0/data/namespace"]
    );
}

#[test]
fn e1_2_4_contract_warning_comes_before_the_namespace_warning() {
    let mut data = extension_data(&json!({}));
    data["namespace"] = json!("semantic_compiler");
    let mut document = with_extension(&data);
    document["contract"] = json!("semantic_compiler.query");
    assert_eq!(
        warning_pointers(&document),
        [
            "/contract",
            "/query/nonportable_extensions/0/data/namespace"
        ]
    );
}

#[test]
fn e1_2_4_warnings_are_reported_only_for_a_query_that_passes_the_json_schema() {
    let mut document = wide_table_query();
    document["contract"] = json!("semantic_compiler.query");
    document["query"]["unknown"] = json!(true);
    assert_eq!(rejected(&document).code, ParseErrorCode::InvalidQuery);
}

#[test]
fn con_0_5_another_namespace_is_invalid_query() {
    let mut data = extension_data(&json!({}));
    data["namespace"] = json!("acme");
    assert_eq!(
        rejected(&with_extension(&data)).code,
        ParseErrorCode::InvalidQuery
    );
}
