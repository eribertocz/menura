//! ARC-6.9: for a sample of the conformance models, every MCP tool returns the exact document
//! `menura` writes to standard output, and `isError` exactly when it exits `1`.
//!
//! The CLI binary is the one `cargo test` builds for the `menura-cli` tests in the same target
//! directory; run the workspace tests (`cargo test`) so it exists.

#![cfg(test)]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "ARC-6.9: the test reads the conformance files, writes option files and runs the CLI"
)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{CONTEXT, QUERY, call, shop_path, texts, workspace};
use serde_json::{Value, json};

/// Every how many conformance models one is sampled.
const SAMPLE_EVERY: usize = 8;

/// The `menura` binary next to this test's target directory.
fn cli() -> PathBuf {
    let executable = std::env::current_exe().unwrap();
    let directory = executable.parent().and_then(Path::parent).unwrap();
    let path = directory.join(format!("menura{}", std::env::consts::EXE_SUFFIX));
    assert!(
        path.exists(),
        "`{}` is missing: run the workspace tests with `cargo test`",
        path.display()
    );
    path
}

/// One argument of a call: its member name, its CLI flag value (a path), and its MCP value.
struct Input {
    member: &'static str,
    path: PathBuf,
    value: Value,
}

fn document(member: &'static str, path: &Path) -> Option<Input> {
    let text = fs::read_to_string(path).ok()?;
    Some(Input {
        member,
        path: path.to_owned(),
        value: Value::String(text),
    })
}

/// Runs `menura <tool> …` and the MCP tool with the same inputs, and compares them.
fn assert_parity(tool: &str, inputs: &[Input]) {
    let mut command = Command::new(cli());
    command.arg(tool.replace('_', "-"));
    let mut arguments = serde_json::Map::new();
    for input in inputs {
        command
            .arg(format!("--{}", input.member.replace('_', "-")))
            .arg(&input.path);
        arguments.insert(input.member.to_owned(), input.value.clone());
    }
    let output = command.output().unwrap();
    let answer = call(tool, &Value::Object(arguments));
    let context = format!(
        "{tool} {:?}",
        inputs.iter().map(|i| &i.path).collect::<Vec<_>>()
    );
    match output.status.code() {
        Some(code @ (0 | 1)) => {
            let (items, is_error) = texts(&answer);
            let stdout = String::from_utf8(output.stdout).unwrap();
            assert_eq!(items[0], stdout.strip_suffix('\n').unwrap(), "{context}");
            assert_eq!(is_error, code == 1, "{context}");
        }
        _ => assert_eq!(answer["error"]["code"], -32602, "{context}: {answer}"),
    }
}

/// The sampled conformance models: their model, catalog schema and first security context.
fn sampled_models() -> Vec<(Input, Input, Option<Input>)> {
    let mut directories: Vec<PathBuf> = fs::read_dir(workspace().join("conformance/models"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    directories.sort();
    directories
        .iter()
        .step_by(SAMPLE_EVERY)
        .filter_map(|directory| {
            let model = document("model", &directory.join("model.yaml"))?;
            let schema = document("catalog_schema", &directory.join("catalog-schema.json"))?;
            let context =
                first_context(directory).and_then(|path| document("security_context", &path));
            Some((model, schema, context))
        })
        .collect()
}

fn first_context(directory: &Path) -> Option<PathBuf> {
    let mut contexts: Vec<PathBuf> = fs::read_dir(directory.join("contexts"))
        .ok()?
        .map(|entry| entry.unwrap().path())
        .collect();
    contexts.sort();
    contexts.into_iter().next()
}

fn renamed(input: &Input, member: &'static str) -> Input {
    Input {
        member,
        path: input.path.clone(),
        value: input.value.clone(),
    }
}

fn copy(input: &Input) -> Input {
    renamed(input, input.member)
}

#[test]
fn arc_6_9_model_tools_equal_the_cli_on_sampled_conformance_models() {
    let models = sampled_models();
    assert!(models.len() > 10);
    for (model, schema, context) in &models {
        for tool in ["validate", "dependencies", "integrity_checks"] {
            assert_parity(tool, &[copy(model), copy(schema)]);
        }
        assert_parity("migrate", &[copy(model)]);
        if let Some(context) = context {
            assert_parity("describe", &[copy(model), copy(schema), copy(context)]);
        }
    }
}

#[test]
fn arc_6_9_diff_equals_the_cli_on_pairs_of_sampled_models() {
    let models = sampled_models();
    for pair in models.windows(2) {
        let [(old_model, old_schema, _), (new_model, new_schema, _)] = pair else {
            continue;
        };
        assert_parity(
            "diff",
            &[
                renamed(old_model, "old_model"),
                renamed(old_schema, "old_catalog_schema"),
                renamed(new_model, "new_model"),
                renamed(new_schema, "new_catalog_schema"),
            ],
        );
    }
}

/// A scratch file of this test with `content`.
fn scratch(test: &str, name: &str, content: &str) -> PathBuf {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("parity")
        .join(test);
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join(name);
    fs::write(&path, content).unwrap();
    path
}

fn shop_inputs(test: &str, query: &str) -> Vec<Input> {
    vec![
        document("model", &shop_path("model.yaml")).unwrap(),
        document("catalog_schema", &shop_path("catalog-schema.json")).unwrap(),
        document("query", &scratch(test, "query.json", query)).unwrap(),
        document("security_context", &scratch(test, "context.json", CONTEXT)).unwrap(),
    ]
}

#[test]
fn arc_6_9_compile_and_explain_equal_the_cli() {
    for (test, query) in [("plan", QUERY), ("error", "{}")] {
        assert_parity("compile", &shop_inputs(test, query));
        assert_parity("explain", &shop_inputs(test, query));
        let mut inputs = shop_inputs(test, query);
        inputs.push(Input {
            member: "compile_options",
            path: scratch(test, "options.json", r#"{"max_rows": 10}"#),
            value: json!({"max_rows": 10}),
        });
        assert_parity("compile", &inputs);
    }
}

#[test]
fn arc_6_9_the_audit_record_equals_the_one_the_cli_writes() {
    for (test, query) in [("audit_plan", QUERY), ("audit_error", "{}")] {
        let inputs = shop_inputs(test, query);
        let options = scratch(test, "options.json", r#"{"audit": true}"#);
        let record = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("parity")
            .join(test)
            .join("record.json");
        let mut command = Command::new(cli());
        command.arg("compile");
        let mut arguments = serde_json::Map::new();
        for input in &inputs {
            command
                .arg(format!("--{}", input.member.replace('_', "-")))
                .arg(&input.path);
            arguments.insert(input.member.to_owned(), input.value.clone());
        }
        command.arg("--compile-options").arg(&options);
        command.arg("--audit-record").arg(&record);
        let output = command.output().unwrap();
        arguments.insert("compile_options".to_owned(), json!({"audit": true}));
        let (items, _) = texts(&call("compile", &Value::Object(arguments)));
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(items[0], stdout.strip_suffix('\n').unwrap());
        let written = fs::read_to_string(&record).unwrap();
        assert_eq!(items[1], written.strip_suffix('\n').unwrap());
    }
}
