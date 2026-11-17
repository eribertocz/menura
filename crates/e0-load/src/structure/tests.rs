//! Validating the structure: E0B-1.1, E0B-1.2, E0B-3.1 to E0B-3.6, E0B-01 to E0B-07, W0B-01,
//! W0B-02, MDL-1.1, OSS-2.1, OSS-2.2 and DEV-7.
//!
//! The model documents of the `e0-structure-*` conformance cases are read here at compile time,
//! so structure validation is checked against the same files the conformance suite uses.

use super::diagnostic::{StructureCode, StructureDiagnostic};
use super::output::{AnsiSqlVariant, ElementExpression, ElementKind, Structure};
use super::schema::OFFICIAL_SCHEMA_SOURCE;
use super::selection::select_model;
use super::validate_structure;
use crate::read::{Node, read_document};

macro_rules! conformance_model {
    ($name:literal) => {
        include_bytes!(concat!(
            "../../../../conformance/models/",
            $name,
            "/model.yaml"
        ))
    };
}

fn read(bytes: &[u8]) -> Node {
    read_document(bytes).expect("the document is read")
}

fn errors(bytes: &[u8], model_name: Option<&str>) -> Vec<StructureDiagnostic> {
    validate_structure(&read(bytes), model_name).expect_err("the structure is rejected")
}

/// The code and JSON Pointer of every rejecting diagnostic.
fn summary(bytes: &[u8]) -> Vec<(&'static str, String)> {
    errors(bytes, None)
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn warnings(structure: &Structure<'_>) -> Vec<(&'static str, String)> {
    structure
        .warnings
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

/// A minimal valid document, with `dataset_lines` added to its only dataset.
fn minimal(dataset_lines: &str) -> String {
    format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    datasets:\n      - name: orders\n        source: shop.main.orders\n{dataset_lines}"
    )
}

const ORDERS: &str = "/semantic_model/0/datasets/0";

#[test]
fn e0b_1_1_the_official_schema_is_embedded_with_its_source() {
    assert_eq!(
        OFFICIAL_SCHEMA_SOURCE.repository,
        "https://github.com/apache/ossie"
    );
    assert_eq!(OFFICIAL_SCHEMA_SOURCE.tag, "osi-0.1.1-rc1");
    assert_eq!(OFFICIAL_SCHEMA_SOURCE.path, "core-spec/osi-schema.json");
    let document = read(minimal("").as_bytes());
    let structure = validate_structure(&document, None).expect("a minimal document is valid");
    assert!(structure.expressions.is_empty());
    assert!(structure.warnings.is_empty());
}

#[test]
fn e0b_3_1_e0b_01_version_other_than_0_1_1() {
    let found = summary(conformance_model!("e0-structure-version-001"));
    assert_eq!(found, [pair("E0B-01", "/version")]);
    let missing = minimal("").replace("version: \"0.1.1\"\n", "");
    assert_eq!(summary(missing.as_bytes()), [pair("E0B-01", "")]);
    let number = minimal("").replace("\"0.1.1\"", "0.1");
    assert_eq!(summary(number.as_bytes()), [pair("E0B-01", "/version")]);
}

#[test]
fn e0b_02_required_property_missing() {
    let found = summary(conformance_model!("e0-structure-required-property-001"));
    assert_eq!(found, [pair("E0B-02", &format!("{ORDERS}/fields/1"))]);
}

#[test]
fn e0b_03_wrong_type() {
    let found = summary(conformance_model!("e0-structure-wrong-type-001"));
    assert_eq!(found, [pair("E0B-03", &format!("{ORDERS}/fields/1/label"))]);
    let ai_context = minimal("        ai_context: 5\n");
    assert_eq!(
        summary(ai_context.as_bytes()),
        [pair("E0B-03", &format!("{ORDERS}/ai_context"))]
    );
    assert_eq!(summary(b"[]"), [pair("E0B-03", "")]);
}

#[test]
fn e0b_1_2_e0b_04_objects_are_closed() {
    let found = summary(conformance_model!("e0-structure-property-not-allowed-001"));
    assert_eq!(found, [pair("E0B-04", &format!("{ORDERS}/owner"))]);
    let dimension = minimal(
        "        fields:\n          - name: day\n            expression: {dialects: [{dialect: ANSI_SQL, expression: day}]}\n            dimension: {is_time: true, grain: day}\n",
    );
    assert_eq!(
        summary(dimension.as_bytes()),
        [pair(
            "E0B-04",
            &format!("{ORDERS}/fields/0/dimension/grain")
        )]
    );
}

#[test]
fn e0b_05_value_outside_an_enumeration() {
    let found = summary(conformance_model!("e0-structure-enumeration-001"));
    assert_eq!(
        found,
        [pair(
            "E0B-05",
            &format!("{ORDERS}/fields/1/expression/dialects/1/dialect")
        )]
    );
    let vendor = minimal("        custom_extensions: [{vendor_name: ACME, data: '{}'}]\n");
    assert_eq!(
        summary(vendor.as_bytes()),
        [pair(
            "E0B-05",
            &format!("{ORDERS}/custom_extensions/0/vendor_name")
        )]
    );
}

#[test]
fn e0b_06_empty_list_where_one_item_is_required() {
    let found = summary(conformance_model!("e0-structure-empty-list-001"));
    assert_eq!(
        found,
        [pair(
            "E0B-06",
            &format!("{ORDERS}/fields/1/expression/dialects")
        )]
    );
    let datasets = "version: \"0.1.1\"\nsemantic_model: [{name: shop, datasets: []}]\n";
    assert_eq!(
        summary(datasets.as_bytes()),
        [pair("E0B-06", "/semantic_model/0/datasets")]
    );
}

#[test]
fn e0b_3_2_e0b_07_dev_7_several_models_need_model_name() {
    let document = read(conformance_model!("e0-structure-several-models-001"));
    let rejected = validate_structure(&document, None).expect_err("model_name is required");
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].code, StructureCode::ModelSelection);
    assert_eq!(rejected[0].pointer.as_str(), "/semantic_model");

    let second = validate_structure(&document, Some("e0_structure_several_models_001_other"))
        .expect("model_name selects the second model");
    assert_eq!(second.semantic_model.index, 1);
    assert_eq!(second.semantic_model.pointer.as_str(), "/semantic_model/1");
    assert!(second.expressions.iter().all(|expression| {
        expression
            .pointer
            .as_str()
            .starts_with("/semantic_model/1/")
    }));

    let unknown = validate_structure(&document, Some("missing")).expect_err("names no model");
    assert_eq!(unknown[0].code, StructureCode::ModelSelection);
}

#[test]
fn e0b_3_2_e0b_07_model_name_naming_two_semantic_models_is_rejected() {
    let document = read(conformance_model!("e0-structure-repeated-model-name-001"));
    let rejected = validate_structure(&document, Some("e0_structure_repeated_model_name_001"))
        .expect_err("model_name names two semantic models");
    assert_eq!(rejected.len(), 1);
    assert_eq!(rejected[0].code, StructureCode::ModelSelection);
    assert_eq!(rejected[0].pointer.as_str(), "/semantic_model");

    let without = validate_structure(&document, None).expect_err("model_name is required");
    assert_eq!(without[0].code, StructureCode::ModelSelection);
}

#[test]
fn e0b_07_a_repeated_name_is_rejected_only_when_model_name_selects_it() {
    let document = "version: \"0.1.1\"\nsemantic_model:\n  - {name: shop, datasets: []}\n  - {name: shop, datasets: []}\n  - {name: other, datasets: []}\n";
    let found = errors(document.as_bytes(), Some("shop"));
    assert_eq!(found[0].code, StructureCode::ModelSelection);
    assert_eq!(found[0].pointer.as_str(), "/semantic_model");
    let other = errors(document.as_bytes(), Some("other"));
    assert!(
        other
            .iter()
            .all(|diagnostic| diagnostic.code != StructureCode::ModelSelection)
    );
}

/// The index `select_model` returns for semantic models with the given names.
fn selected(names: &[&str], model_name: &str) -> Result<Option<usize>, StructureCode> {
    let models = names
        .iter()
        .map(|name| format!("  - {{name: '{name}', datasets: []}}\n"))
        .collect::<Vec<_>>()
        .concat();
    let document = read(format!("version: \"0.1.1\"\nsemantic_model:\n{models}").as_bytes());
    select_model(&document, Some(model_name)).map_err(|diagnostic| diagnostic.code)
}

#[test]
fn e0b_3_2_a_regular_model_name_matches_case_insensitively() {
    assert_eq!(selected(&["other", "SHOP"], "shop"), Ok(Some(1)));
    assert_eq!(selected(&["other", "shop"], "Shop"), Ok(Some(1)));
    assert_eq!(
        selected(&["other", "r\u{e9}gion"], "R\u{c9}GION"),
        Ok(Some(1))
    );
}

#[test]
fn e0b_3_2_e0b_07_names_equal_after_normalization_are_ambiguous() {
    assert_eq!(
        selected(&["shop", "SHOP"], "shop"),
        Err(StructureCode::ModelSelection)
    );
    assert_eq!(
        selected(&["shop", "\"SHOP\""], "SHOP"),
        Err(StructureCode::ModelSelection)
    );
}

#[test]
fn e0b_3_2_a_delimited_model_name_is_compared_exactly() {
    assert_eq!(selected(&["\"Shop\"", "shop"], "\"Shop\""), Ok(Some(0)));
    assert_eq!(selected(&["\"Shop\"", "shop"], "\"SHOP\""), Ok(Some(1)));
    assert_eq!(
        selected(&["\"Shop\"", "other"], "shop"),
        Err(StructureCode::ModelSelection)
    );
    assert_eq!(
        selected(&["\"Shop\"", "other"], "Shop"),
        Err(StructureCode::ModelSelection)
    );
}

#[test]
fn e0b_3_2_doubled_quotes_in_a_delimited_name_stand_for_one_quote() {
    assert_eq!(
        selected(&["other", "\"a\"\"b\""], "\"a\"\"b\""),
        Ok(Some(1))
    );
    assert_eq!(
        selected(&["other", "\"a\"\"b\""], "\"a\"\"\"\"b\""),
        Err(StructureCode::ModelSelection)
    );
}

#[test]
fn e0b_07_model_name_must_name_the_only_model() {
    let document = read(minimal("").as_bytes());
    assert!(validate_structure(&document, Some("shop")).is_ok());
    let rejected = validate_structure(&document, Some("other")).expect_err("names no model");
    assert_eq!(rejected[0].code, StructureCode::ModelSelection);
}

#[test]
fn e0b_3_2_e0b_07_a_document_without_semantic_models_is_rejected() {
    let document = read(conformance_model!("e0-structure-no-semantic-model-001"));
    for model_name in [None, Some("shop")] {
        let rejected = validate_structure(&document, model_name).expect_err("there is no model");
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].code, StructureCode::ModelSelection);
        assert_eq!(rejected[0].pointer.as_str(), "/semantic_model");
    }
}

#[test]
fn e0b_3_6_diagnostics_are_in_document_order_then_by_code() {
    let found = summary(conformance_model!("e0-structure-document-order-001"));
    assert_eq!(
        found,
        [
            pair("E0B-03", &format!("{ORDERS}/fields/1/label")),
            pair("E0B-04", &format!("{ORDERS}/fields/1/comment")),
        ]
    );
    let several = "version: \"0.2\"\nsemantic_model: [{datasets: []}, {}]\nextra: 1\n";
    assert_eq!(
        summary(several.as_bytes()),
        [
            pair("E0B-01", "/version"),
            pair("E0B-07", "/semantic_model"),
            pair("E0B-02", "/semantic_model/0"),
            pair("E0B-06", "/semantic_model/0/datasets"),
            pair("E0B-02", "/semantic_model/1"),
            pair("E0B-04", "/extra"),
        ]
    );
}

#[test]
fn e0b_3_6_diagnostics_carry_the_location_of_the_node() {
    let document = minimal("        owner: sales\n");
    let found = errors(document.as_bytes(), None);
    assert_eq!((found[0].location.line, found[0].location.column), (7, 16));
}

#[test]
fn e0b_3_4_w0b_01_other_dialects_are_kept_and_not_compiled() {
    let document = read(conformance_model!("e0-structure-variants"));
    let structure = validate_structure(&document, None).expect("the model loads");
    let status = &structure.expressions[1];
    assert_eq!(status.kind, ElementKind::Field);
    assert_eq!(status.pointer.to_string(), format!("{ORDERS}/fields/1"));
    assert_eq!(status.variant, AnsiSqlVariant::Single("status"));
    assert!(warnings(&structure).contains(&pair(
        "W0B-01",
        &format!("{ORDERS}/fields/1/expression/dialects/1")
    )));
}

#[test]
fn e0b_3_3_w0b_02_oss_2_2_a_field_without_ansi_sql_is_unusable() {
    let document = read(conformance_model!("e0-structure-variants"));
    let structure = validate_structure(&document, None).expect("the model loads");
    let status_code = &structure.expressions[3];
    assert_eq!(
        status_code.pointer.to_string(),
        format!("{ORDERS}/fields/3")
    );
    assert_eq!(status_code.variant, AnsiSqlVariant::Unusable);
    assert!(!status_code.is_usable());
    assert_eq!(ElementExpression::UNUSABLE_QUERY_ERROR, "E0B-08");
    assert_eq!(
        warnings(&structure),
        [
            pair(
                "W0B-01",
                &format!("{ORDERS}/fields/1/expression/dialects/1")
            ),
            pair("W0B-02", &format!("{ORDERS}/fields/3/expression")),
            pair(
                "W0B-01",
                &format!("{ORDERS}/fields/3/expression/dialects/0")
            ),
            pair(
                "W0B-01",
                &format!("{ORDERS}/fields/3/expression/dialects/1")
            ),
        ]
    );
    let usable = structure
        .expressions
        .iter()
        .filter(|e| e.is_usable())
        .count();
    assert_eq!(usable, structure.expressions.len() - 1);
}

#[test]
fn e0b_3_3_w0b_02_two_ansi_sql_variants_make_a_metric_unusable() {
    let document = format!(
        "{}    metrics:\n      - name: total\n        expression:\n          dialects:\n            - {{dialect: ANSI_SQL, expression: SUM(orders.amount)}}\n            - {{dialect: ANSI_SQL, expression: SUM(orders.total)}}\n",
        minimal("")
    );
    let document = read(document.as_bytes());
    let structure = validate_structure(&document, None).expect("the model loads");
    assert_eq!(structure.expressions[0].kind, ElementKind::Metric);
    assert_eq!(structure.expressions[0].variant, AnsiSqlVariant::Unusable);
    assert_eq!(
        warnings(&structure),
        [pair("W0B-02", "/semantic_model/0/metrics/0/expression")]
    );
}

#[test]
fn e0b_3_5_oss_2_1_ai_context_dialects_and_vendors_are_kept() {
    let document = read(conformance_model!("e0-structure-variants"));
    let structure = validate_structure(&document, None).expect("the model loads");
    let codes: Vec<StructureCode> = structure.warnings.iter().map(|w| w.code).collect();
    assert!(codes.iter().all(|code| code.is_warning()));
    assert_eq!(structure.expressions.len(), 9);
}

#[test]
fn mdl_1_1_the_sample_model_has_a_valid_structure() {
    let document = read(conformance_model!("sample"));
    let structure = validate_structure(&document, None).expect("the sample model loads");
    assert!(structure.warnings.is_empty());
}
