//! `menura compile`, `explain`, `describe`, `integrity-checks` and `migrate`: ARC-5.4,
//! ARC-5.7 to ARC-5.10.

#![cfg(test)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: the test runs the command-line wrapper as a process and writes its input files"
)]

use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

const QUERY: &str = r#"{"contract": "menura.query", "contract_version": "1.1.0",
    "time_zone": "UTC", "query": {"dimensions": ["orders.region"]}}"#;
const CONTEXT: &str = r#"{"contract": "menura.security_context", "contract_version": "1.1.0",
    "subject": "user-1", "roles": ["admin"], "attributes": {}}"#;

fn shop(file: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/shop")
        .join(file)
        .display()
        .to_string()
}

/// A file of the test's own folder under the target directory, with `content`.
fn scratch_file(test: &str, name: &str, content: &[u8]) -> io::Result<String> {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    fs::create_dir_all(&folder)?;
    let path = folder.join(name);
    fs::write(&path, content)?;
    Ok(path.display().to_string())
}

fn scratch_path(test: &str, name: &str) -> String {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join(name);
    let _ = fs::remove_file(&path);
    path.display().to_string()
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_menura"))
        .args(arguments)
        .output()
        .unwrap()
}

fn document(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

fn assert_usage_error(output: &Output) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

/// The arguments of `command` over the shop model, with `extra`.
fn compile_line(test: &str, command: &str, extra: &[&str]) -> Vec<String> {
    let query = scratch_file(test, "query.json", QUERY.as_bytes()).unwrap();
    let context = scratch_file(test, "context.json", CONTEXT.as_bytes()).unwrap();
    let mut line: Vec<String> = [
        command,
        "--model",
        &shop("model.yaml"),
        "--catalog-schema",
        &shop("catalog-schema.json"),
        "--query",
        &query,
        "--security-context",
        &context,
    ]
    .iter()
    .map(|argument| (*argument).to_owned())
    .collect();
    line.extend(extra.iter().map(|argument| (*argument).to_owned()));
    line
}

fn run_line(line: &[String]) -> Output {
    let arguments: Vec<&str> = line.iter().map(String::as_str).collect();
    run(&arguments)
}

#[test]
fn arc_5_7_compile_writes_the_compiled_plan_and_exits_0() {
    let output = run_line(&compile_line("compile_plan", "compile", &[]));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let plan = document(&output);
    assert_eq!(plan["contract"], "menura.compiled_plan");
    assert!(!plan["sql"].as_str().unwrap().is_empty());
}

#[test]
fn arc_5_7_explain_writes_the_explanation_and_exits_0() {
    let output = run_line(&compile_line("explain_plan", "explain", &[]));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let explanation = document(&output);
    assert_eq!(explanation["contract"], "menura.explanation");
    assert_eq!(explanation["sql"], "");
}

#[test]
fn arc_5_7_a_query_error_is_the_compile_error_with_exit_1() {
    let test = "compile_error";
    let query = scratch_file(test, "bad-query.json", b"{}").unwrap();
    let mut line = compile_line(test, "compile", &[]);
    line[6] = query;
    let output = run_line(&line);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(document(&output)["contract"], "menura.compile_error");
}

#[test]
fn arc_5_7_compile_options_are_read() {
    let test = "compile_options";
    let options = scratch_file(test, "options.json", br#"{"timeout_ms": 1234}"#).unwrap();
    let output = run_line(&compile_line(
        test,
        "compile",
        &["--compile-options", &options],
    ));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(document(&output)["limits"]["timeout_ms"], 1234);
}

#[test]
fn arc_5_7_with_audit_compile_writes_the_audit_record() {
    let test = "compile_audit";
    let options = scratch_file(test, "options.json", br#"{"audit": true}"#).unwrap();
    let record = scratch_path(test, "record.json");
    let output = run_line(&compile_line(
        test,
        "compile",
        &["--compile-options", &options, "--audit-record", &record],
    ));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(document(&output)["contract"], "menura.compiled_plan");
    let written: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(written["contract"], "menura.audit_record");
}

#[test]
fn arc_5_7_with_audit_a_compile_error_also_writes_the_audit_record() {
    let test = "compile_audit_error";
    let options = scratch_file(test, "options.json", br#"{"audit": true}"#).unwrap();
    let query = scratch_file(test, "bad-query.json", b"{}").unwrap();
    let record = scratch_path(test, "record.json");
    let mut line = compile_line(
        test,
        "compile",
        &["--compile-options", &options, "--audit-record", &record],
    );
    line[6] = query;
    let output = run_line(&line);
    assert_eq!(output.status.code(), Some(1));
    let written: Value = serde_json::from_slice(&fs::read(&record).unwrap()).unwrap();
    assert_eq!(written["contract"], "menura.audit_record");
}

#[test]
fn arc_5_7_a_model_that_does_not_load_gives_no_audit_record() {
    let test = "compile_audit_no_load";
    let options = scratch_file(test, "options.json", br#"{"audit": true}"#).unwrap();
    let model = scratch_file(test, "model.yaml", b"version: [").unwrap();
    let record = scratch_path(test, "record.json");
    let mut line = compile_line(
        test,
        "compile",
        &["--compile-options", &options, "--audit-record", &record],
    );
    line[2] = model;
    let output = run_line(&line);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(document(&output)["contract"], "menura.compile_error");
    assert!(!Path::new(&record).exists());
}

#[test]
fn arc_5_7_a_record_that_cannot_be_written_exits_2() {
    let test = "compile_audit_unwritable";
    let options = scratch_file(test, "options.json", br#"{"audit": true}"#).unwrap();
    let record = scratch_path(test, "missing-folder/record.json");
    let output = run_line(&compile_line(
        test,
        "compile",
        &["--compile-options", &options, "--audit-record", &record],
    ));
    assert_usage_error(&output);
}

#[test]
fn arc_5_7_every_usage_error_exits_2() {
    let test = "compile_usage";
    let audit = scratch_file(test, "audit.json", br#"{"audit": true}"#).unwrap();
    let plain = scratch_file(test, "plain.json", br"{}").unwrap();
    let unknown = scratch_file(test, "unknown.json", br#"{"dialects": "duckdb"}"#).unwrap();
    let wrong = scratch_file(test, "wrong.json", br#"{"max_rows": null}"#).unwrap();
    let list = scratch_file(test, "list.json", b"[]").unwrap();
    let record = scratch_path(test, "record.json");
    for extra in [
        vec!["--compile-options", &audit],
        vec!["--compile-options", &plain, "--audit-record", &record],
        vec!["--audit-record", &record],
        vec!["--compile-options", &unknown],
        vec!["--compile-options", &wrong],
        vec!["--compile-options", &list],
        vec!["--compile-options", "/nonexistent/options.json"],
        vec!["--compile-options"],
        vec!["--query", "again.json"],
        vec!["--compile-options=x"],
        vec!["positional"],
    ] {
        assert_usage_error(&run_line(&compile_line(test, "compile", &extra)));
    }
    let mut missing = compile_line(test, "compile", &[]);
    missing.drain(5..7);
    assert_usage_error(&run_line(&missing));
    assert_usage_error(&run_line(&compile_line(
        test,
        "explain",
        &["--audit-record", &record],
    )));
}

#[test]
fn arc_5_7_explain_ignores_audit_and_actor() {
    let test = "explain_audit";
    let options = scratch_file(test, "options.json", br#"{"audit": true, "actor": "a"}"#).unwrap();
    let output = run_line(&compile_line(
        test,
        "explain",
        &["--compile-options", &options],
    ));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
}

#[test]
fn arc_5_8_describe_writes_the_model_description() {
    let context = scratch_file("describe", "context.json", CONTEXT.as_bytes()).unwrap();
    let output = run(&[
        "describe",
        "--security-context",
        &context,
        "--model",
        &shop("model.yaml"),
        "--catalog-schema",
        &shop("catalog-schema.json"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(document(&output)["contract"], "menura.model_description");
}

#[test]
fn arc_5_8_describe_with_a_bad_security_context_exits_1() {
    let context = scratch_file("describe_error", "context.json", b"{}").unwrap();
    let output = run(&[
        "describe",
        "--security-context",
        &context,
        "--model",
        &shop("model.yaml"),
        "--catalog-schema",
        &shop("catalog-schema.json"),
    ]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(document(&output)["contract"], "menura.compile_error");
}

#[test]
fn arc_5_8_integrity_checks_writes_its_document_and_refuses_other_dialects() {
    let model = shop("model.yaml");
    let catalog = shop("catalog-schema.json");
    let base = [
        "integrity-checks",
        "--model",
        &model,
        "--catalog-schema",
        &catalog,
    ];
    let output = run(&base);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(document(&output)["contract"], "menura.integrity_checks");
    let mut duckdb = base.to_vec();
    duckdb.extend(["--dialect", "duckdb"]);
    assert_eq!(run(&duckdb).stdout, output.stdout);
    let mut other = base.to_vec();
    other.extend(["--dialect", "postgres"]);
    assert_usage_error(&run(&other));
}

#[test]
fn arc_5_8_integrity_checks_accepts_the_clickhouse_dialect() {
    let model = shop("model.yaml");
    let catalog = shop("catalog-schema.json");
    let output = run(&[
        "integrity-checks",
        "--model",
        &model,
        "--catalog-schema",
        &catalog,
        "--dialect",
        "clickhouse",
    ]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(document(&output)["dialect"], "clickhouse");
    let other = run(&[
        "integrity-checks",
        "--model",
        &model,
        "--catalog-schema",
        &catalog,
        "--dialect",
        "postgres",
    ]);
    assert_usage_error(&other);
    let reason = String::from_utf8(other.stderr).unwrap();
    assert!(
        reason.contains("unknown dialect `postgres`; the dialects are `clickhouse` and `duckdb`"),
        "{reason}"
    );
}

#[test]
fn arc_5_7_compile_accepts_the_clickhouse_dialect_option() {
    let test = "compile_clickhouse";
    let options = scratch_file(test, "options.json", br#"{"dialect": "clickhouse"}"#).unwrap();
    let output = run_line(&compile_line(
        test,
        "compile",
        &["--compile-options", &options],
    ));
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let plan = document(&output);
    assert_eq!(plan["dialect"], "clickhouse");
    assert!(plan["sql"].as_str().unwrap().contains("SETTINGS"));
}

#[test]
fn arc_5_9_migrate_writes_the_migration_report() {
    let model = scratch_file(
        "migrate",
        "model.yaml",
        b"version: \"0.1.1\"\nsemantic_model:\n  - name: m\n    datasets:\n      - name: d\n        source: s.t\n        custom_extensions:\n          - vendor_name: COMMON\n            data: '{\"namespace\": \"semantic_compiler\", \"version\": 1}'\n",
    )
    .unwrap();
    let output = run(&["migrate", "--model", &model]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report = document(&output);
    assert_eq!(report["contract"], "menura.migration_report");
    assert_eq!(report["changes"][0]["to"], "menura");
    let options = scratch_file("migrate", "options.json", br#"{"namespace": "acme"}"#).unwrap();
    let output = run(&["migrate", "--load-options", &options, "--model", &model]);
    assert_eq!(document(&output)["changes"][0]["to"], "acme");
}

#[test]
fn arc_5_9_a_model_e0_rejects_exits_1_and_catalog_flags_are_usage_errors() {
    let model = scratch_file("migrate_error", "model.yaml", b"version: [").unwrap();
    let output = run(&["migrate", "--model", &model]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(document(&output)["contract"], "menura.compile_error");
    assert_usage_error(&run(&[
        "migrate",
        "--model",
        &model,
        "--catalog-schema",
        &model,
    ]));
    assert_usage_error(&run(&["migrate", "--model", "/nonexistent/model.yaml"]));
}

#[test]
fn arc_5_10_a_document_that_is_not_utf8_is_a_diagnostic_not_a_usage_error() {
    let model = scratch_file("migrate_utf8", "model.yaml", b"version: \xff").unwrap();
    let output = run(&["migrate", "--model", &model]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(document(&output)["errors"][0]["code"], "E0A-01");
}

#[test]
fn arc_5_4_help_lists_every_command() {
    let output = run(&["migrate", "--help"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8(output.stdout).unwrap();
    for command in [
        "validate",
        "compile",
        "explain",
        "describe",
        "integrity-checks",
        "dependencies",
        "migrate",
        "diff",
    ] {
        assert!(text.contains(&format!("menura {command} ")), "{command}");
    }
}

#[test]
fn mig_3_3a_migrate_with_a_number_without_a_json_form_exits_1_with_emig_01() {
    let model = scratch_file(
        "migrate_emig",
        "model.yaml",
        b"version: 0.3.0.dev0\nname: m\ndatasets:\n  - name: d\n    source: s.t\n    nonportable_extensions:\n      - vendor_name: COMMON\n        data: {limit: .nan}\n",
    )
    .unwrap();
    let options = scratch_file(
        "migrate_emig",
        "options.json",
        br#"{"ossie_format": "development"}"#,
    )
    .unwrap();
    let output = run(&["migrate", "--model", &model, "--load-options", &options]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(document(&output)["errors"][0]["code"], "EMIG-01");
}
