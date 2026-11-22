//! `migrate` (MIG-1 to MIG-3): the deprecated forms of a model document rewritten, each
//! rewrite reported, and nothing else changed.

#![cfg(test)]

use menura_contracts::types::migration_report::MigrationReportDocument;
use menura_contracts::{ContractKind, canonical_json, validate_structure};
use menura_core::{
    CompileErrorDocument, LoadOptions, MigrationRule, OssieFormat, migrate, migration_report_json,
};
use serde_json::Value;

const FORMER: &str = r#"'{"namespace": "semantic_compiler", "version": 1}'"#;
const CURRENT: &str = r#"'{"namespace": "menura", "version": 1}'"#;

/// A release-format model whose `orders` dataset carries one `COMMON` entry with `data`.
fn release(data: &str) -> String {
    format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    datasets:\n      - name: orders\n        source: shop.main.orders\n        custom_extensions:\n          - vendor_name: COMMON\n            data: {data}\n"
    )
}

fn report(model: &str, options: &LoadOptions) -> MigrationReportDocument {
    migrate(model.as_bytes(), options).expect("the model is migrated")
}

fn error(model: &str) -> CompileErrorDocument {
    migrate(model.as_bytes(), &LoadOptions::default()).expect_err("the model is rejected")
}

fn json(report: &MigrationReportDocument) -> Value {
    serde_json::from_str(&migration_report_json(report).unwrap()).unwrap()
}

fn pointers(report: &MigrationReportDocument) -> Vec<&str> {
    report
        .changes
        .iter()
        .map(|change| change.pointer.as_str())
        .collect()
}

#[test]
fn mig_1_migrate_reads_only_the_model() {
    let report = report(&release(FORMER), &LoadOptions::default());
    assert_eq!(report.changes.len(), 1);
}

#[test]
fn mig_2_the_report_has_its_contract_and_validates_against_its_schema() {
    let report = report(&release(FORMER), &LoadOptions::default());
    let document = json(&report);
    assert_eq!(document["contract"], "menura.migration_report");
    assert_eq!(
        validate_structure(ContractKind::MigrationReport, &document),
        Ok(())
    );
    let change = &report.changes[0];
    assert_eq!(change.rule, MigrationRule::FormerNamespace);
    assert_eq!(document["changes"][0]["rule"], "MIG-3.2a");
    assert_eq!(
        (change.from.as_str(), change.to.as_str()),
        ("semantic_compiler", "menura")
    );
}

#[test]
fn mig_3_1_every_model_is_covered_and_model_name_is_ignored() {
    let one = release(FORMER);
    let second = one
        .split_once("  - name: shop\n")
        .map(|(_, model)| format!("  - name: other\n{model}"))
        .unwrap();
    let options = LoadOptions {
        model_name: Some("absent".to_owned()),
        ..LoadOptions::default()
    };
    let report = report(&format!("{one}{second}"), &options);
    assert_eq!(
        pointers(&report),
        [
            "/semantic_model/0/datasets/0/custom_extensions/0/data",
            "/semantic_model/1/datasets/0/custom_extensions/0/data",
        ]
    );
}

#[test]
fn mig_3_1_a_document_e0a_or_e0b_rejects_returns_its_compile_error() {
    let unreadable = error("version: [");
    assert_eq!(unreadable.errors[0].code, "E0A-02");
    let invalid = error("version: \"0.1.1\"\n");
    assert!(invalid.errors[0].code.starts_with("E0B-"), "{invalid:?}");
    assert!(invalid.warnings.is_empty());
}

#[test]
fn mig_3_1_checks_of_later_substages_do_not_reject() {
    // An unknown extension key is an E0.c error; migrate reads only E0.a and E0.b.
    let data = r#"'{"namespace": "semantic_compiler", "version": 1, "unknown": 1}'"#;
    assert_eq!(
        report(&release(data), &LoadOptions::default())
            .changes
            .len(),
        1
    );
}

#[test]
fn mig_3_2a_the_release_format_replaces_only_the_namespace_token() {
    let data = r#"'{ "version" : 1,"namespace":"semantic_compiler" , "x": "semantic_compiler"}'"#;
    let document = json(&report(&release(data), &LoadOptions::default()));
    assert_eq!(
        document["model"]["semantic_model"][0]["datasets"][0]["custom_extensions"][0]["data"],
        r#"{ "version" : 1,"namespace":"menura" , "x": "semantic_compiler"}"#
    );
}

#[test]
fn mig_3_2a_the_development_format_points_at_the_namespace_member() {
    let model = "version: 0.3.0.dev0\nname: shop\ndatasets:\n  - name: orders\n    source: shop.main.orders\n    nonportable_extensions:\n      - vendor_name: COMMON\n        data: {namespace: semantic_compiler, version: 1}\n";
    let options = LoadOptions {
        ossie_format: OssieFormat::Development,
        ..LoadOptions::default()
    };
    let report = report(model, &options);
    assert_eq!(
        pointers(&report),
        ["/datasets/0/nonportable_extensions/0/data/namespace"]
    );
    let document = json(&report);
    let data = &document["model"]["datasets"][0]["nonportable_extensions"][0]["data"];
    assert_eq!(data["namespace"], "menura");
    assert_eq!(data["version"], 1);
}

#[test]
fn mig_3_2a_the_configured_namespace_is_written() {
    let options = LoadOptions {
        namespace: "acme".to_owned(),
        ..LoadOptions::default()
    };
    let report = report(&release(FORMER), &options);
    assert_eq!(report.changes[0].to, "acme");
    assert_eq!(
        json(&report)["model"]["semantic_model"][0]["datasets"][0]["custom_extensions"][0]["data"],
        r#"{"namespace": "acme", "version": 1}"#
    );
}

#[test]
fn mig_3_2a_with_the_former_namespace_configured_nothing_is_rewritten() {
    let options = LoadOptions {
        namespace: "semantic_compiler".to_owned(),
        ..LoadOptions::default()
    };
    assert!(report(&release(FORMER), &options).changes.is_empty());
}

#[test]
fn mig_3_2a_every_other_entry_is_left_unchanged() {
    for data in [
        CURRENT,
        r#"'{"namespace": "semantic_compiler", "namespace": "semantic_compiler"}'"#,
        r#"'{"inner": {"namespace": "semantic_compiler"}}'"#,
        r"'not json'",
        r#"'["semantic_compiler"]'"#,
    ] {
        let model = release(data);
        let report = report(&model, &LoadOptions::default());
        assert!(report.changes.is_empty(), "{data}");
    }
    let other_vendor = release(FORMER).replace("vendor_name: COMMON", "vendor_name: DBT");
    assert!(
        report(&other_vendor, &LoadOptions::default())
            .changes
            .is_empty()
    );
}

#[test]
fn mig_3_3_an_alias_expanded_twice_gives_two_changes() {
    let model = format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    custom_extensions: &entries\n      - vendor_name: COMMON\n        data: {FORMER}\n    datasets:\n      - name: orders\n        source: shop.main.orders\n        custom_extensions: *entries\n"
    );
    let report = report(&model, &LoadOptions::default());
    assert_eq!(
        pointers(&report),
        [
            "/semantic_model/0/custom_extensions/0/data",
            "/semantic_model/0/datasets/0/custom_extensions/0/data",
        ]
    );
}

#[test]
fn mig_3_3_members_keep_their_order_and_the_format_is_kept() {
    let model = "{\"version\": \"0.1.1\", \"semantic_model\": [{\"name\": \"shop\", \"description\": 2026-01-02, \"datasets\": [{\"source\": \"s.t\", \"name\": \"d\"}]}]}";
    let report = report(model, &LoadOptions::default());
    assert!(report.changes.is_empty());
    let text = serde_json::to_string(&report.model).unwrap();
    assert_eq!(
        text,
        r#"{"version":"0.1.1","semantic_model":[{"name":"shop","description":"2026-01-02","datasets":[{"source":"s.t","name":"d"}]}]}"#
    );
}

#[test]
fn mig_3_4_changes_are_in_document_order() {
    let model = format!(
        "version: \"0.1.1\"\nsemantic_model:\n  - name: shop\n    metrics:\n      - name: m\n        expression: {{dialects: [{{dialect: ANSI_SQL, expression: COUNT(*)}}]}}\n        custom_extensions:\n          - vendor_name: COMMON\n            data: {FORMER}\n    datasets:\n      - name: orders\n        source: shop.main.orders\n        custom_extensions:\n          - vendor_name: DBT\n            data: x\n          - vendor_name: COMMON\n            data: {FORMER}\n"
    );
    let report = report(&model, &LoadOptions::default());
    assert_eq!(
        pointers(&report),
        [
            "/semantic_model/0/metrics/0/custom_extensions/0/data",
            "/semantic_model/0/datasets/0/custom_extensions/1/data",
        ]
    );
}

#[test]
fn mig_3_4_a_document_without_deprecated_forms_is_returned_as_read() {
    let report = report(&release(CURRENT), &LoadOptions::default());
    assert!(report.changes.is_empty());
    assert_eq!(
        json(&report)["model"]["semantic_model"][0]["datasets"][0]["custom_extensions"][0]["data"],
        r#"{"namespace": "menura", "version": 1}"#
    );
}

#[test]
fn mig_3_5_migrating_the_model_of_a_report_gives_no_change() {
    let first = report(&release(FORMER), &LoadOptions::default());
    let model = serde_json::to_string(&first.model).unwrap();
    let second = report(&model, &LoadOptions::default());
    assert!(second.changes.is_empty());
    assert_eq!(second.model, first.model);
}

#[test]
fn mig_3_6_the_same_document_and_options_give_the_same_report() {
    let model = release(FORMER);
    let texts: Vec<String> = (0..2)
        .map(|_| canonical_json(&json(&report(&model, &LoadOptions::default()))))
        .collect();
    assert_eq!(texts[0], texts[1]);
    let pretty: Vec<String> = (0..2)
        .map(|_| migration_report_json(&report(&model, &LoadOptions::default())).unwrap())
        .collect();
    assert_eq!(pretty[0], pretty[1]);
}

/// A development-format model whose `orders` entry has `data` written `data`.
fn development(data: &str) -> String {
    format!(
        "version: 0.3.0.dev0\nname: shop\ndatasets:\n  - name: orders\n    source: shop.main.orders\n    nonportable_extensions:\n      - vendor_name: COMMON\n        data: {data}\n"
    )
}

fn development_options() -> LoadOptions {
    LoadOptions {
        ossie_format: OssieFormat::Development,
        ..LoadOptions::default()
    }
}

#[test]
fn mig_3_3a_integers_of_any_size_are_written_with_their_exact_digits() {
    let digits = "123456789012345678901234567890123456789012345";
    let model = development(&format!("{{namespace: other, size: {digits}, small: -7}}"));
    let report = report(&model, &development_options());
    let text = migration_report_json(&report).unwrap();
    assert!(text.contains(&format!("\"size\": {digits},")), "{text}");
    assert!(text.contains("\"small\": -7"), "{text}");
}

#[test]
fn mig_3_3a_a_number_without_a_json_form_is_refused_with_emig_01() {
    let model = development("{namespace: semantic_compiler, version: 1, limit: .inf}");
    let error = migrate(model.as_bytes(), &development_options()).unwrap_err();
    assert_eq!(error.errors.len(), 1);
    assert_eq!(error.errors[0].code, "EMIG-01");
    assert_eq!(
        error.errors[0].location.as_ref().unwrap().pointer,
        "/datasets/0/nonportable_extensions/0/data/limit"
    );
    assert!(error.warnings.is_empty());
}

#[test]
fn mig_3_3_the_report_text_keeps_the_member_order_of_the_model() {
    let text = migration_report_json(&report(&release(FORMER), &LoadOptions::default())).unwrap();
    let model = text.find("\"model\"").unwrap();
    assert!(text.find("\"contract\"").unwrap() < text.find("\"changes\"").unwrap());
    assert!(
        text[model..].find("\"version\"").unwrap()
            < text[model..].find("\"semantic_model\"").unwrap()
    );
    assert!(text.find("\"pointer\"").unwrap() < text.find("\"rule\"").unwrap());
}
