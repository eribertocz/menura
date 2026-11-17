//! Validating the development-line format: MDL-1.1 (`ossie_format: development`), MDL-2.4,
//! OSS-5.1, OSS-5.2, `W0B-03` and `E0B-09`.
//!
//! The model of the `e0-development-*` conformance cases is read here at compile time, with
//! `ossie_format: development`. Cases with `load_options` (CS-2.7) follow once the runner reads
//! them (#115).

use semantic_compiler_contracts::types::audit_record::OssieFormat;

use super::diagnostic::{StructureCode, StructureDiagnostic};
use super::output::{CompiledVariant, DeclaredDatatype, ElementKind, LogicalKind, Structure};
use super::schema::DEVELOPMENT_SCHEMA_SOURCE;
use super::validate_structure;
use crate::read::{Node, read_document};

pub(super) const DEVELOPMENT_MODEL: &[u8] =
    include_bytes!("../../../../conformance/models/e0-development-format/model.yaml");

pub(super) fn read(bytes: &[u8]) -> Node {
    read_document(bytes).expect("the document is read")
}

pub(super) fn load(document: &Node) -> Structure<'_> {
    validate_structure(document, OssieFormat::Development, None).expect("the model loads")
}

pub(super) fn summary(found: &[StructureDiagnostic]) -> Vec<(&'static str, String)> {
    found
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect()
}

pub(super) fn errors(text: &str, format: OssieFormat) -> Vec<(&'static str, String)> {
    let document = read(text.as_bytes());
    summary(&validate_structure(&document, format, None).expect_err("the structure is rejected"))
}

pub(super) fn pair(code: &'static str, pointer: &str) -> (&'static str, String) {
    (code, pointer.to_owned())
}

/// A minimal development-line document, with `field_lines` added to its only field.
fn minimal(field_lines: &str) -> String {
    format!(
        "version: 0.3.0.dev0\nname: shop\ndatasets:\n  - name: orders\n    source: shop.main.orders\n    fields:\n      - name: amount\n{field_lines}"
    )
}

pub(super) fn with_sql(field_lines: &str) -> String {
    minimal(&format!(
        "        expression: {{sql: amount}}\n{field_lines}"
    ))
}

#[test]
fn oss_5_1_the_development_schema_is_embedded_with_its_source() {
    assert_eq!(
        DEVELOPMENT_SCHEMA_SOURCE.repository,
        "https://github.com/apache/ossie"
    );
    assert_eq!(DEVELOPMENT_SCHEMA_SOURCE.tag, None);
    assert_eq!(
        DEVELOPMENT_SCHEMA_SOURCE.commit,
        "aa500d37894c3d62c96cdce4192cbecea4d7f06a"
    );
    assert_eq!(
        DEVELOPMENT_SCHEMA_SOURCE.path,
        "core-spec/ossie-schema.json"
    );
}

#[test]
fn mdl_1_1_oss_5_1_a_development_format_model_loads() {
    use CompiledVariant::{AnsiSql, OssieSql2026};
    use ElementKind::{DatasetMetric, Field, Metric};
    let document = read(DEVELOPMENT_MODEL);
    let structure = load(&document);
    assert_eq!(structure.format, OssieFormat::Development);
    assert_eq!(structure.semantic_model.index, 0);
    assert_eq!(structure.semantic_model.pointer.as_str(), "");
    let expressions: Vec<(ElementKind, &str, CompiledVariant<'_>)> = structure
        .expressions
        .iter()
        .map(|expression| {
            (
                expression.kind,
                expression.pointer.as_str(),
                expression.variant,
            )
        })
        .collect();
    assert_eq!(
        expressions,
        [
            (Field, "/datasets/0/fields/0", OssieSql2026("order_id")),
            (Field, "/datasets/0/fields/1", AnsiSql("status")),
            (Field, "/datasets/0/fields/2", OssieSql2026("amount")),
            (
                DatasetMetric,
                "/datasets/0/metrics/0",
                OssieSql2026("COUNT(*)")
            ),
            (Field, "/datasets/1/fields/0", OssieSql2026("branch")),
            (Field, "/datasets/1/fields/1", OssieSql2026("customer_id")),
            (Field, "/datasets/1/fields/2", OssieSql2026("balance")),
            (Metric, "/metrics/0", OssieSql2026("SUM(orders.amount)")),
            (Metric, "/metrics/1", OssieSql2026("SUM(balances.balance)")),
        ]
    );
    assert_eq!(
        summary(&structure.warnings),
        [
            pair("W0B-01", "/datasets/0/fields/1/expression/dialects/1"),
            pair("W0B-03", "/datasets/0/fields/2/datatype"),
        ]
    );
}

#[test]
fn oss_5_2_datatype_values_map_to_logical_kinds() {
    let document = read(DEVELOPMENT_MODEL);
    let structure = load(&document);
    let datatypes: Vec<DeclaredDatatype<'_>> = structure
        .expressions
        .iter()
        .map(|expression| expression.datatype)
        .collect();
    assert_eq!(
        datatypes,
        [
            DeclaredDatatype::Mapped(LogicalKind::Integer),
            DeclaredDatatype::Mapped(LogicalKind::Text),
            DeclaredDatatype::Unknown("Opaque"),
            DeclaredDatatype::Mapped(LogicalKind::Integer),
            DeclaredDatatype::NotDeclared,
            DeclaredDatatype::Mapped(LogicalKind::Integer),
            DeclaredDatatype::Mapped(LogicalKind::Decimal),
            DeclaredDatatype::Mapped(LogicalKind::Float),
            DeclaredDatatype::Mapped(LogicalKind::Decimal),
        ]
    );
}

#[test]
fn oss_5_2_every_datatype_of_the_table_maps_to_its_logical_kind() {
    let table = [
        ("String", LogicalKind::Text),
        ("Integer", LogicalKind::Integer),
        ("Decimal", LogicalKind::Decimal),
        ("Float", LogicalKind::Float),
        ("Boolean", LogicalKind::Boolean),
        ("Date", LogicalKind::Date),
        (
            "DateTime",
            LogicalKind::Timestamp {
                with_time_zone: false,
            },
        ),
        (
            "DateTimeTz",
            LogicalKind::Timestamp {
                with_time_zone: true,
            },
        ),
        ("Time", LogicalKind::Time),
    ];
    for (datatype, kind) in table {
        let document = read(with_sql(&format!("        datatype: {datatype}\n")).as_bytes());
        let structure = load(&document);
        assert_eq!(
            structure.expressions[0].datatype,
            DeclaredDatatype::Mapped(kind)
        );
        assert!(structure.expressions[0].is_usable());
        assert!(structure.warnings.is_empty());
    }
}

#[test]
fn w0b_03_e0b_09_an_unknown_datatype_makes_the_element_unusable() {
    let document = read(DEVELOPMENT_MODEL);
    let structure = load(&document);
    let amount = &structure.expressions[2];
    assert_eq!(amount.pointer.as_str(), "/datasets/0/fields/2");
    assert!(!amount.is_usable());
    assert_eq!(amount.query_error(), Some("E0B-09"));
    let usable = structure
        .expressions
        .iter()
        .filter(|expression| expression.is_usable())
        .count();
    assert_eq!(usable, structure.expressions.len() - 1);
    assert!(StructureCode::UnknownDatatype.is_warning());
}

#[test]
fn w0b_03_e0b_08_comes_before_e0b_09_for_an_element_with_both_causes() {
    let text = minimal(
        "        expression: {dialects: [{dialect: SNOWFLAKE, expression: amount}]}\n        datatype: Opaque\n",
    );
    let document = read(text.as_bytes());
    let structure = load(&document);
    assert_eq!(structure.expressions[0].query_error(), Some("E0B-08"));
    let field = "/datasets/0/fields/0";
    assert_eq!(
        summary(&structure.warnings),
        [
            pair("W0B-02", &format!("{field}/expression")),
            pair("W0B-01", &format!("{field}/expression/dialects/0")),
            pair("W0B-03", &format!("{field}/datatype")),
        ]
    );
}

#[test]
fn oss_5_2_a_datatype_outside_the_schema_enumeration_is_rejected() {
    // OSS-5.2: the schema decides first, so a value outside its enumeration is `E0B-05`.
    assert_eq!(
        errors(
            &with_sql("        datatype: Money\n"),
            OssieFormat::Development
        ),
        [pair("E0B-05", "/datasets/0/fields/0/datatype")]
    );
}

#[test]
fn oss_5_1_sql_is_compiled_when_dialects_are_also_given() {
    // Section 4.5, `expression` with both `sql` and `dialects`: `sql` is compiled, `W0B-01` for
    // each entry in a dialect other than `ANSI_SQL`, and no `W0B-02`.
    let text = minimal(
        "        expression:\n          sql: amount\n          dialects:\n            - {dialect: ANSI_SQL, expression: total}\n            - {dialect: SNOWFLAKE, expression: total}\n",
    );
    let document = read(text.as_bytes());
    let structure = load(&document);
    assert_eq!(
        structure.expressions[0].variant,
        CompiledVariant::OssieSql2026("amount")
    );
    assert_eq!(
        summary(&structure.warnings),
        [pair("W0B-01", "/datasets/0/fields/0/expression/dialects/1")]
    );
}

#[test]
fn oss_5_1_an_ossie_sql_dialect_entry_is_read_as_in_0_1_1() {
    let text = minimal(
        "        expression: {dialects: [{dialect: OSSIE_SQL_2026, expression: amount}]}\n",
    );
    let document = read(text.as_bytes());
    let structure = load(&document);
    assert_eq!(structure.expressions[0].variant, CompiledVariant::Unusable);
    let field = "/datasets/0/fields/0";
    assert_eq!(
        summary(&structure.warnings),
        [
            pair("W0B-02", &format!("{field}/expression")),
            pair("W0B-01", &format!("{field}/expression/dialects/0")),
        ]
    );
}

#[test]
fn oss_5_1_an_expression_without_sql_or_dialects_misses_a_property() {
    assert_eq!(
        errors(
            &minimal("        expression: {}\n"),
            OssieFormat::Development
        ),
        [pair("E0B-02", "/datasets/0/fields/0/expression")]
    );
}

#[test]
fn oss_5_1_e0b_01_the_development_version_is_required() {
    let text = with_sql("").replace("0.3.0.dev0", "\"0.1.1\"");
    assert_eq!(
        errors(&text, OssieFormat::Development),
        [pair("E0B-01", "/version")]
    );
}

#[test]
fn mdl_1_1_the_development_document_without_the_option_fails() {
    let found = errors(
        std::str::from_utf8(DEVELOPMENT_MODEL).expect("the model is UTF-8"),
        OssieFormat::Release,
    );
    assert_eq!(found.first(), Some(&pair("E0B-02", "")));
    assert!(found.contains(&pair("E0B-01", "/version")));
    assert!(found.contains(&pair("E0B-04", "/datasets")));
    assert!(found.contains(&pair("E0B-04", "/nonportable_extensions")));
}

#[test]
fn mdl_1_1_a_release_document_fails_with_the_development_option() {
    let text = "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    datasets: [{name: orders, source: shop.main.orders}]\n";
    assert_eq!(
        errors(text, OssieFormat::Development),
        [
            pair("E0B-02", ""),
            pair("E0B-01", "/version"),
            pair("E0B-04", "/semantic_model"),
        ]
    );
}

#[test]
fn e0b_07_model_name_must_name_the_development_model() {
    let document = read(with_sql("").as_bytes());
    assert!(validate_structure(&document, OssieFormat::Development, Some("shop")).is_ok());
    let rejected = validate_structure(&document, OssieFormat::Development, Some("other"))
        .expect_err("names no model");
    assert_eq!(summary(&rejected), [pair("E0B-07", "")]);
}

#[test]
fn e0b_3_2_model_name_is_matched_as_an_identifier() {
    let regular = read(with_sql("").as_bytes());
    for model_name in ["shop", "SHOP", "Shop", "\"SHOP\""] {
        assert!(
            validate_structure(&regular, OssieFormat::Development, Some(model_name)).is_ok(),
            "{model_name} names the model shop"
        );
    }
    let rejected = validate_structure(&regular, OssieFormat::Development, Some("\"shop\""))
        .expect_err("a delimited name is compared exactly with the normalized SHOP");
    assert_eq!(summary(&rejected), [pair("E0B-07", "")]);

    let delimited = read(
        with_sql("")
            .replace("name: shop", "name: '\"Shop\"'")
            .as_bytes(),
    );
    assert!(validate_structure(&delimited, OssieFormat::Development, Some("\"Shop\"")).is_ok());
    for model_name in ["shop", "SHOP", "Shop"] {
        let rejected = validate_structure(&delimited, OssieFormat::Development, Some(model_name))
            .expect_err("a regular name never matches the delimited Shop");
        assert_eq!(summary(&rejected), [pair("E0B-07", "")]);
    }
}

#[test]
fn e0b_3_2_e0b_07_a_malformed_delimited_model_name_names_no_development_model() {
    for model_name in ["\"a\"b\"", "\"", "\"\"", "\"a"] {
        let document = read(
            with_sql("")
                .replace("name: shop", &format!("name: '{model_name}'"))
                .as_bytes(),
        );
        let rejected = validate_structure(&document, OssieFormat::Development, Some(model_name))
            .expect_err("a malformed model_name names no model");
        assert_eq!(summary(&rejected), [pair("E0B-07", "")], "{model_name}");
    }
}
