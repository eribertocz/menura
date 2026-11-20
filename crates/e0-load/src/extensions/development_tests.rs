//! The extension rules on development-line `nonportable_extensions` entries: MDL-2.4, E0C-2.8,
//! and `E0C-05` for `derived` on a dataset metric (DER-5.1.1).
//!
//! Each `*-development-*` conformance model is the release model of the same name written in
//! the development format, so both must give the same codes, at the matching members.

use menura_contracts::types::audit_record::OssieFormat;

use super::output::{Extensions, ExtensionsFailure, ObjectKind};
use super::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::structure::validate_structure;

macro_rules! conformance_model {
    ($name:literal) => {
        include_bytes!(concat!(
            "../../../../conformance/models/",
            $name,
            "/model.yaml"
        ))
    };
}

fn validate(bytes: &[u8], format: OssieFormat) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(bytes).expect("the document is read");
    let structure = validate_structure(&document, format, None).expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
}

fn development(bytes: &[u8]) -> Result<Extensions, ExtensionsFailure> {
    validate(bytes, OssieFormat::Development)
}

/// The code and JSON Pointer of every rejecting diagnostic.
fn errors(found: Result<Extensions, ExtensionsFailure>) -> Vec<(&'static str, String)> {
    found
        .expect_err("the extensions are rejected")
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

/// The release diagnostics written with the development line's members: a flat root and
/// `nonportable_extensions` lists.
fn as_development(found: Vec<(&'static str, String)>) -> Vec<(&'static str, String)> {
    found
        .into_iter()
        .map(|(code, pointer)| {
            let pointer = pointer
                .replacen("/semantic_model/0", "", 1)
                .replace("/custom_extensions/", "/nonportable_extensions/");
            (code, pointer)
        })
        .collect()
}

/// A development-line model whose orders dataset carries one `COMMON` entry with `data`.
fn on_orders(data: &str) -> String {
    format!(
        "version: 0.3.0.dev0\nname: shop\ndatasets:\n  - name: orders\n    source: shop.main.orders\n    nonportable_extensions:\n      - vendor_name: COMMON\n        data: {data}\n"
    )
}

const ORDERS_DATA: &str = "/datasets/0/nonportable_extensions/0/data";

#[test]
fn mdl_2_4_development_entries_give_the_codes_of_their_release_entries() {
    let pairs: [(&[u8], &[u8]); 7] = [
        (
            conformance_model!("e0-extensions-unsupported-version-001"),
            conformance_model!("e0-extensions-development-unsupported-version-001"),
        ),
        (
            conformance_model!("e0-extensions-unknown-key-001"),
            conformance_model!("e0-extensions-development-unknown-key-001"),
        ),
        (
            conformance_model!("e0-extensions-two-entries-001"),
            conformance_model!("e0-extensions-development-two-entries-001"),
        ),
        (
            conformance_model!("e0-extensions-wrong-object-001"),
            conformance_model!("e0-extensions-development-wrong-object-001"),
        ),
        (
            conformance_model!("e0-populations-wrong-value-001"),
            conformance_model!("e0-populations-development-wrong-value-001"),
        ),
        (
            conformance_model!("e0-additivity-non-additive-key-001"),
            conformance_model!("e0-additivity-development-non-additive-key-001"),
        ),
        (
            conformance_model!("e0-derived-wrong-value-001"),
            conformance_model!("e0-derived-development-wrong-value-001"),
        ),
    ];
    for (release, development_model) in pairs {
        let expected = as_development(errors(validate(release, OssieFormat::Release)));
        assert!(!expected.is_empty());
        assert_eq!(errors(development(development_model)), expected);
    }
}

#[test]
fn mdl_2_4_development_entries_are_read_into_the_same_extensions() {
    let extensions = development(conformance_model!("e0-development-format")).expect("accepted");
    let summary: Vec<_> = extensions
        .entries
        .iter()
        .map(|entry| {
            (
                entry.object,
                entry.data_pointer.to_string(),
                entry.extensions.keys().cloned().collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(
        summary[0],
        (
            ObjectKind::Model,
            "/nonportable_extensions/0/data".to_owned(),
            vec!["security_context".to_owned()]
        )
    );
    assert_eq!(
        summary[1],
        (
            ObjectKind::Dataset,
            ORDERS_DATA.to_owned(),
            vec!["access".to_owned()]
        )
    );
    assert!(extensions.warnings.is_empty());
    let derived = development(conformance_model!("e0-derived-development")).expect("accepted");
    assert_eq!(derived.entries.len(), 2);
}

#[test]
fn der_5_1_1_e0c_05_derived_on_a_dataset_metric() {
    let found = errors(development(conformance_model!(
        "e0-derived-development-dataset-metric-001"
    )));
    assert_eq!(
        found,
        [pair(
            "E0C-05",
            "/datasets/0/metrics/0/nonportable_extensions/0/data"
        )]
    );
}

#[test]
fn e0c_2_3_additivity_is_valid_on_a_dataset_metric_and_context_is_not() {
    let model = |data: &str| {
        format!(
            "version: 0.3.0.dev0\nname: shop\ndatasets:\n  - name: orders\n    source: shop.main.orders\n    fields:\n      - {{name: amount, expression: {{sql: amount}}}}\n    metrics:\n      - name: total\n        expression: {{sql: SUM(amount)}}\n        nonportable_extensions:\n          - vendor_name: COMMON\n            data: {data}\n"
        )
    };
    let additive = model("{namespace: menura, version: 1, additivity: {type: non_additive}}");
    let extensions = development(additive.as_bytes()).expect("accepted");
    assert_eq!(extensions.entries[0].object, ObjectKind::DatasetMetric);
    assert_eq!(
        extensions.additivity[0].metric.to_string(),
        "/datasets/0/metrics/0"
    );
    let context = model("{namespace: menura, version: 1, context: {}}");
    assert_eq!(
        errors(development(context.as_bytes())),
        [pair(
            "E0C-05",
            "/datasets/0/metrics/0/nonportable_extensions/0/data"
        )]
    );
}

#[test]
fn e0c_2_8_e0c_03_data_without_a_json_form() {
    for model in [
        conformance_model!("e0-extensions-development-key-not-a-string-001").as_slice(),
        conformance_model!("e0-extensions-development-number-without-json-form-001").as_slice(),
    ] {
        let found = errors(development(model));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "E0C-03");
        assert!(found[0].1.ends_with("/nonportable_extensions/0/data"));
    }
    for data in [
        "{namespace: menura, version: 1, ~: x}",
        "{namespace: menura, version: 1, access: {true: x}}",
        "{namespace: menura, version: 1, access: [.nan]}",
        "{namespace: menura, version: 1, access: -.inf}",
        "{namespace: menura, version: .inf}",
    ] {
        let document = on_orders(data);
        assert_eq!(
            errors(development(document.as_bytes())),
            [pair("E0C-03", ORDERS_DATA)],
            "{data}"
        );
    }
}

#[test]
fn e0c_2_8_an_entry_of_another_namespace_is_kept_whatever_its_values() {
    let document = on_orders("{namespace: acme, version: 1, 1: .inf}");
    let extensions = development(document.as_bytes()).expect("accepted");
    assert!(extensions.entries.is_empty());
    assert!(extensions.warnings.is_empty());
}

#[test]
fn e0c_2_8_integers_give_the_code_of_the_same_digits_in_the_release_format() {
    for version in ["0", "2", "-1", "123456789012345678901234567890"] {
        let document = on_orders(&format!("{{namespace: menura, version: {version}}}"));
        assert_eq!(
            errors(development(document.as_bytes())),
            [pair("E0C-01", ORDERS_DATA)],
            "{version}"
        );
    }
    for version in ["1", "0x1", "+1"] {
        let document = on_orders(&format!(
            "{{namespace: menura, version: {version}, access: {{mode: authenticated}}}}"
        ));
        assert_eq!(
            development(document.as_bytes())
                .expect("accepted")
                .entries
                .len(),
            1,
            "{version}"
        );
    }
    let text = on_orders("{namespace: menura, version: '1'}");
    assert_eq!(
        errors(development(text.as_bytes())),
        [pair("E0C-01", ORDERS_DATA)]
    );
}

#[test]
fn e0c_2_8_a_yaml_null_is_the_json_null() {
    let metric = |division: &str| {
        format!(
            "version: 0.3.0.dev0\nname: shop\ndatasets:\n  - name: orders\n    source: shop.main.orders\nmetrics:\n  - name: rate\n    expression: {{sql: AGG(a) / AGG(b)}}\n    nonportable_extensions:\n      - vendor_name: COMMON\n        data: {{namespace: menura, version: 1, derived: {{on_division_by_zero: {division}}}}}\n"
        )
    };
    let unquoted = metric("null");
    assert_eq!(
        errors(development(unquoted.as_bytes())),
        [pair("E0C-03", "/metrics/0/nonportable_extensions/0/data")]
    );
    let quoted = metric("'null'");
    assert_eq!(
        development(quoted.as_bytes())
            .expect("accepted")
            .derived
            .len(),
        1
    );
}

#[test]
fn con_0_5_w0c_02_a_development_entry_under_the_former_namespace_is_ours() {
    let former =
        on_orders("{namespace: semantic_compiler, version: 1, access: {mode: authenticated}}");
    let extensions = development(former.as_bytes()).expect("accepted");
    assert_eq!(extensions.entries.len(), 1);
    let warnings: Vec<_> = extensions
        .warnings
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect();
    assert_eq!(warnings, [pair("W0C-02", ORDERS_DATA)]);
}
