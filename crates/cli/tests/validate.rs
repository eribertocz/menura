//! `menura validate` and its alias `semantic-compiler`: ARC-5.4, ARC-5.5 and CON-0.5.

#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: the test runs the command-line wrapper as a process and writes its input files"
)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn conformance_model(name: &str, file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/models")
        .join(name)
        .join(file)
}

fn sample(file: &str) -> String {
    conformance_model("sample", file).display().to_string()
}

/// A file of the test's own folder under the target directory, with `content`.
fn scratch_file(test: &str, name: &str, content: &[u8]) -> io::Result<String> {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join(test);
    fs::create_dir_all(&folder)?;
    let path = folder.join(name);
    fs::write(&path, content)?;
    Ok(path.display().to_string())
}

fn run(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_menura"))
        .args(arguments)
        .output()
}

/// Runs the former binary name, an alias of `menura` until 1.0.0 (CON-0.5).
fn run_alias(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_semantic-compiler"))
        .args(arguments)
        .output()
}

fn report(output: &Output) -> serde_json::Result<serde_json::Value> {
    serde_json::from_slice(&output.stdout)
}

fn assert_usage_error(output: &Output) {
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn arc_5_4_a_valid_model_writes_a_report_without_errors_and_exits_0() {
    let (model, catalog) = (sample("model.yaml"), sample("catalog-schema.json"));
    let output = run(&["validate", "--model", &model, "--catalog-schema", &catalog]).unwrap();
    assert_eq!(output.status.code(), Some(0));
    let report = report(&output).unwrap();
    assert_eq!(report["contract"], "menura.validation_report");
    assert_eq!(report["errors"], serde_json::json!([]));
}

#[test]
fn arc_5_5_the_flags_may_come_in_any_order() {
    let (model, catalog) = (sample("model.yaml"), sample("catalog-schema.json"));
    let output = run(&["validate", "--catalog-schema", &catalog, "--model", &model]).unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn arc_5_4_warnings_do_not_change_the_exit_status() {
    let model = fs::read_to_string(sample("model.yaml")).unwrap().replacen(
        "          - name: amount\n",
        "          - name: discount\n            expression: {dialects: [{dialect: ANSI_SQL, expression: discount}]}\n          - name: amount\n",
        1,
    );
    let model = scratch_file("warnings", "model.yaml", model.as_bytes()).unwrap();
    let catalog = sample("catalog-schema.json");
    let output = run(&["validate", "--model", &model, "--catalog-schema", &catalog]).unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(report(&output).unwrap()["warnings"][0]["code"], "W0D-01");
}

#[test]
fn arc_5_4_a_model_with_errors_writes_the_report_and_exits_1() {
    let model = conformance_model("sample-missing-source", "model.yaml");
    let catalog = conformance_model("sample-missing-source", "catalog-schema.json");
    let output = run(&[
        "validate",
        "--model",
        &model.display().to_string(),
        "--catalog-schema",
        &catalog.display().to_string(),
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let report = report(&output).unwrap();
    assert_eq!(report["contract"], "menura.validation_report");
    assert_eq!(report["errors"][0]["code"], "E0D-02");
}

#[test]
fn arc_5_5_a_catalog_schema_that_is_not_json_is_ecat_01_with_exit_1() {
    let catalog = scratch_file("not-json", "catalog-schema.json", b"not json").unwrap();
    let model = sample("model.yaml");
    let output = run(&["validate", "--model", &model, "--catalog-schema", &catalog]).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report(&output).unwrap()["errors"][0]["code"], "ECAT-01");
}

#[test]
fn arc_5_5_a_model_that_is_not_utf8_is_a_diagnostic_not_a_usage_error() {
    let model = scratch_file("not-utf8", "model.yaml", &[0xff, 0xfe]).unwrap();
    let catalog = sample("catalog-schema.json");
    let output = run(&["validate", "--model", &model, "--catalog-schema", &catalog]).unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report(&output).unwrap()["errors"][0]["code"], "E0A-01");
}

#[test]
fn arc_5_5_the_load_options_file_is_read() {
    let options = scratch_file("options", "options.json", br#"{"model_name": "nowhere"}"#).unwrap();
    let (model, catalog) = (sample("model.yaml"), sample("catalog-schema.json"));
    let output = run(&[
        "validate",
        "--model",
        &model,
        "--load-options",
        &options,
        "--catalog-schema",
        &catalog,
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report(&output).unwrap()["errors"][0]["code"], "E0B-07");
}

/// `validate` with the sample model and catalog schema, then `extra`.
fn validate_with(extra: &[&str]) -> io::Result<Output> {
    let (model, catalog) = (sample("model.yaml"), sample("catalog-schema.json"));
    let mut arguments = vec!["validate", "--model", &model, "--catalog-schema", &catalog];
    arguments.extend_from_slice(extra);
    run(&arguments)
}

#[test]
fn arc_5_5_a_command_line_outside_the_syntax_exits_2_with_nothing_on_standard_output() {
    let model = sample("model.yaml");
    for arguments in [
        &[][..],
        &["compile"][..],
        &["validate"][..],
        &["validate", "--model", &model][..],
        &["validate", "--model", &model, "--catalog-schema"][..],
    ] {
        assert_usage_error(&run(arguments).unwrap());
    }
    for extra in [
        &["--model", &model][..],
        &["--dialect", "duckdb"][..],
        &["extra"][..],
    ] {
        assert_usage_error(&validate_with(extra).unwrap());
    }
}

#[test]
fn arc_5_5_a_bad_load_options_file_or_an_unreadable_file_exits_2() {
    let bad_options = scratch_file("usage", "options.json", br#"{"dialect": "duckdb"}"#).unwrap();
    let not_an_object = scratch_file("usage", "list.json", b"[]").unwrap();
    assert_usage_error(&validate_with(&["--load-options", &bad_options]).unwrap());
    assert_usage_error(&validate_with(&["--load-options", &not_an_object]).unwrap());
    let missing = format!("{}/missing.yaml", env!("CARGO_TARGET_TMPDIR"));
    let catalog = sample("catalog-schema.json");
    let output = run(&[
        "validate",
        "--model",
        &missing,
        "--catalog-schema",
        &catalog,
    ])
    .unwrap();
    assert_usage_error(&output);
}

#[test]
fn arc_5_5_help_writes_the_usage_to_standard_output_and_exits_0() {
    for arguments in [&["--help"][..], &["validate", "--help"][..]] {
        let output = run(arguments).unwrap();
        assert_eq!(output.status.code(), Some(0));
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("menura validate --model <path> --catalog-schema <path>"));
    }
}

#[test]
fn con_0_5_the_former_binary_name_gives_the_same_results() {
    let (model, catalog) = (sample("model.yaml"), sample("catalog-schema.json"));
    let not_json = scratch_file("con_0_5_alias", "catalog.json", b"[").unwrap();
    let command_lines: [&[&str]; 4] = [
        &["validate", "--model", &model, "--catalog-schema", &catalog],
        &["validate", "--model", &model, "--catalog-schema", &not_json],
        &["validate", "--model", &model],
        &["--help"],
    ];
    for arguments in command_lines {
        let (menura, alias) = (run(arguments).unwrap(), run_alias(arguments).unwrap());
        assert_eq!(menura.status.code(), alias.status.code(), "{arguments:?}");
        assert_eq!(menura.stdout, alias.stdout, "{arguments:?}");
        assert_eq!(menura.stderr, alias.stderr, "{arguments:?}");
    }
}
