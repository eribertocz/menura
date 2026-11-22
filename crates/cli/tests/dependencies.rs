//! `menura dependencies` and `menura diff`: ARC-5.6 and DEP-4.4.

#![expect(
    clippy::disallowed_types,
    reason = "A2: the test runs the command-line wrapper as a process"
)]

use std::io;
use std::path::Path;
use std::process::{Command, Output};

fn conformance_file(model: &str, file: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/models")
        .join(model)
        .join(file)
        .display()
        .to_string()
}

fn run(arguments: &[&str]) -> io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_menura"))
        .args(arguments)
        .output()
}

fn document(output: &Output) -> serde_json::Result<serde_json::Value> {
    serde_json::from_slice(&output.stdout)
}

#[test]
fn arc_5_6_dependencies_writes_the_model_dependency_document_and_exits_0() {
    let model = conformance_file("pln-lineage", "model.yaml");
    let catalog = conformance_file("pln-lineage", "catalog-schema.json");
    let output = run(&[
        "dependencies",
        "--catalog-schema",
        &catalog,
        "--model",
        &model,
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let written = document(&output).unwrap();
    assert_eq!(written["contract"], "menura.model_dependencies");
    assert_eq!(written["datasets"][1]["object"], "dataset:ORDERS");
    let again = run(&[
        "dependencies",
        "--model",
        &model,
        "--catalog-schema",
        &catalog,
    ])
    .unwrap();
    assert_eq!(output.stdout, again.stdout);
}

#[test]
fn arc_5_6_a_model_that_does_not_load_gives_its_compile_error_and_exits_1() {
    let model = conformance_file("sample", "catalog-schema.json");
    let catalog = conformance_file("sample", "catalog-schema.json");
    let output = run(&[
        "dependencies",
        "--model",
        &model,
        "--catalog-schema",
        &catalog,
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        document(&output).unwrap()["contract"],
        "menura.compile_error"
    );
}

#[test]
fn dep_4_4_diff_of_the_same_model_has_no_changes() {
    let model = conformance_file("pln-lineage", "model.yaml");
    let catalog = conformance_file("pln-lineage", "catalog-schema.json");
    let output = run(&[
        "diff",
        "--old-model",
        &model,
        "--old-catalog-schema",
        &catalog,
        "--new-model",
        &model,
        "--new-catalog-schema",
        &catalog,
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let written = document(&output).unwrap();
    assert_eq!(written["contract"], "menura.model_diff");
    assert_eq!(written["changes"], serde_json::json!([]));
    assert_eq!(written["old_model_hash"], written["new_model_hash"]);
}

#[test]
fn dep_4_4_diff_lists_the_elements_two_models_differ_by() {
    let old = conformance_file("sample", "model.yaml");
    let new = conformance_file("pln-lineage", "model.yaml");
    let old_catalog = conformance_file("sample", "catalog-schema.json");
    let new_catalog = conformance_file("pln-lineage", "catalog-schema.json");
    let output = run(&[
        "diff",
        "--new-model",
        &new,
        "--new-catalog-schema",
        &new_catalog,
        "--old-model",
        &old,
        "--old-catalog-schema",
        &old_catalog,
    ])
    .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let written = document(&output).unwrap();
    let changes = written["changes"].as_array().unwrap();
    assert!(changes.iter().any(|change| change["change"] == "added"));
}

#[test]
fn arc_5_6_diff_without_a_required_flag_is_a_usage_error() {
    let model = conformance_file("pln-lineage", "model.yaml");
    let output = run(&["diff", "--old-model", &model]).unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
