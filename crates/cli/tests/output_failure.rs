//! ARC-5.4: a command that cannot write its output to standard output exits `2`, with a message
//! on standard error.

#![cfg(test)]
#![cfg(target_os = "linux")]
#![expect(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: the test runs the command-line wrapper as a process and writes its input files"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const QUERY: &str = r#"{"contract": "menura.query", "contract_version": "1.1.0",
    "time_zone": "UTC", "query": {"dimensions": ["orders.region"]}}"#;
const CONTEXT: &str = r#"{"contract": "menura.security_context", "contract_version": "1.1.0",
    "subject": "user-1", "roles": ["admin"], "attributes": {}}"#;

fn shop(file: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../core/tests/fixtures/shop")
        .join(file)
}

/// The `compile` command line over the shop model, with its query and context written under
/// the target directory.
fn compile_line() -> Vec<PathBuf> {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("output_failure");
    fs::create_dir_all(&folder).unwrap();
    let query = folder.join("query.json");
    fs::write(&query, QUERY).unwrap();
    let context = folder.join("context.json");
    fs::write(&context, CONTEXT).unwrap();
    vec![
        "compile".into(),
        "--model".into(),
        shop("model.yaml"),
        "--catalog-schema".into(),
        shop("catalog-schema.json"),
        "--query".into(),
        query,
        "--security-context".into(),
        context,
    ]
}

/// Runs `menura` with `arguments` and its standard output on `/dev/full`, where every write
/// fails.
fn run_on_a_full_device(arguments: &[PathBuf]) -> Output {
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    Command::new(env!("CARGO_BIN_EXE_menura"))
        .args(arguments)
        .stdout(full)
        .output()
        .unwrap()
}

#[test]
fn arc_5_4_a_command_that_cannot_write_its_output_exits_2_with_a_message() {
    for line in [compile_line(), vec![PathBuf::from("--help")]] {
        let output = run_on_a_full_device(&line);
        assert_eq!(output.status.code(), Some(2), "{line:?}: {output:?}");
        let message = String::from_utf8(output.stderr).unwrap();
        assert!(
            message.starts_with("menura: cannot write standard output"),
            "{message}"
        );
    }
}
