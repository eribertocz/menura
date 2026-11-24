//! Reading the development-line `nonportable_extensions` entries (MDL-2.4).

use menura_contracts::types::audit_record::OssieFormat;

use super::development_tests::{DEVELOPMENT_MODEL, errors, load, pair, read, with_sql};
use super::output::ExtensionOwner;
use super::validate_structure;
use crate::read::Value;

#[test]
fn mdl_2_4_nonportable_extensions_are_read_with_object_data() {
    let document = read(DEVELOPMENT_MODEL);
    let structure = load(&document);
    let entries: Vec<(ExtensionOwner, String, String, &str)> = structure
        .nonportable_extensions
        .iter()
        .map(|entry| {
            (
                entry.owner,
                entry.owner_pointer.to_string(),
                entry.pointer.to_string(),
                entry.vendor_name,
            )
        })
        .collect();
    assert_eq!(
        entries,
        [
            (
                ExtensionOwner::Model,
                String::new(),
                "/nonportable_extensions/0".to_owned(),
                "COMMON"
            ),
            (
                ExtensionOwner::Dataset,
                "/datasets/0".to_owned(),
                "/datasets/0/nonportable_extensions/0".to_owned(),
                "COMMON"
            ),
            (
                ExtensionOwner::Dataset,
                "/datasets/1".to_owned(),
                "/datasets/1/nonportable_extensions/0".to_owned(),
                "COMMON"
            ),
            (
                ExtensionOwner::Metric,
                "/metrics/1".to_owned(),
                "/metrics/1/nonportable_extensions/0".to_owned(),
                "ACME"
            ),
        ]
    );
    assert!(
        structure
            .nonportable_extensions
            .iter()
            .all(|entry| matches!(entry.data.value, Value::Mapping(_)))
    );
}

#[test]
fn mdl_2_4_extensions_of_every_object_are_read() {
    let entry = "[{vendor_name: COMMON, data: {namespace: menura, version: 1}}]";
    let text = format!(
        "version: 0.3.0.dev0\nname: shop\ndatasets:\n  - name: orders\n    source: shop.orders\n    fields:\n      - {{name: a, expression: {{sql: a}}, nonportable_extensions: {entry}}}\n    metrics:\n      - {{name: n, expression: {{sql: COUNT(*)}}, nonportable_extensions: {entry}}}\nrelationships:\n  - {{name: r, from: orders, to: orders, from_columns: [a], to_columns: [a], nonportable_extensions: {entry}}}\n"
    );
    let document = read(text.as_bytes());
    let structure = load(&document);
    let owners: Vec<(ExtensionOwner, String)> = structure
        .nonportable_extensions
        .iter()
        .map(|entry| (entry.owner, entry.owner_pointer.to_string()))
        .collect();
    assert_eq!(
        owners,
        [
            (ExtensionOwner::Field, "/datasets/0/fields/0".to_owned()),
            (
                ExtensionOwner::DatasetMetric,
                "/datasets/0/metrics/0".to_owned()
            ),
            (ExtensionOwner::Relationship, "/relationships/0".to_owned()),
        ]
    );
}

#[test]
fn mdl_2_4_the_development_form_needs_the_option_and_object_data() {
    let release = "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    datasets:\n      - name: orders\n        source: shop.orders\n        nonportable_extensions: [{vendor_name: COMMON, data: {namespace: menura, version: 1}}]\n";
    assert_eq!(
        errors(release, OssieFormat::Release),
        [pair(
            "E0B-04",
            "/semantic_model/0/datasets/0/nonportable_extensions"
        )]
    );
    let text_data = with_sql(
        "        nonportable_extensions: [{vendor_name: COMMON, data: '{\"namespace\": \"menura\", \"version\": 1}'}]\n",
    );
    assert_eq!(
        errors(&text_data, OssieFormat::Development),
        [pair(
            "E0B-03",
            "/datasets/0/fields/0/nonportable_extensions/0/data"
        )]
    );
    let custom = with_sql(
        "        custom_extensions: [{vendor_name: COMMON, data: '{\"namespace\": \"menura\", \"version\": 1}'}]\n",
    );
    assert_eq!(
        errors(&custom, OssieFormat::Development),
        [pair("E0B-04", "/datasets/0/fields/0/custom_extensions")]
    );
}

#[test]
fn oss_5_1_a_release_model_keeps_no_nonportable_extensions() {
    let text = "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    datasets: [{name: orders, source: shop.orders}]\n";
    let document = read(text.as_bytes());
    let structure = validate_structure(&document, OssieFormat::Release, None).expect("loads");
    assert_eq!(structure.format, OssieFormat::Release);
    assert!(structure.nonportable_extensions.is_empty());
}
