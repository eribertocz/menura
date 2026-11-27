//! PLT-1.4: `menura-mcp` writes its answers with line feed endings only, on every platform,
//! whatever line endings its client sends.

#![cfg(test)]
#![expect(
    clippy::disallowed_types,
    reason = "PLT-1.4: the test runs the server as a process over standard input and output"
)]

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use common::{shop, shop_compile_arguments};
use serde_json::{Value, json};

/// The bytes the server writes for `messages`, each sent as one line ending with `ending`.
fn served(messages: &[Value], ending: &str) -> Vec<u8> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_menura-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for message in messages {
        stdin
            .write_all(format!("{message}{ending}").as_bytes())
            .unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");
    output.stdout
}

/// Requests whose answers are a result, a tool result over several lines of text, a tool
/// error and a JSON-RPC error.
fn messages() -> Vec<Value> {
    vec![
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "compile", "arguments": shop_compile_arguments()}}),
        json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call",
            "params": {"name": "migrate", "arguments": {"model": shop("model.yaml")}}}),
        json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call",
            "params": {"name": "validate", "arguments": {"model": "a: [",
                "catalog_schema": shop("catalog-schema.json")}}}),
        json!({"jsonrpc": "2.0", "id": 6, "method": "resources/list"}),
    ]
}

#[test]
fn plt_1_4_every_answer_ends_with_a_line_feed_and_no_carriage_return() {
    let output = served(&messages(), "\n");
    assert!(!output.contains(&b'\r'));
    assert!(output.ends_with(b"\n"));
    let lines: Vec<&[u8]> = output.split(|byte| *byte == b'\n').collect();
    assert_eq!(lines.len(), messages().len() + 1);
    assert_eq!(lines.last(), Some(&&b""[..]));
}

#[test]
fn plt_1_4_a_client_sending_crlf_gets_line_feed_endings_and_the_same_answers() {
    let line_feeds = served(&messages(), "\n");
    let carriage_returns = served(&messages(), "\r\n");
    assert!(!carriage_returns.contains(&b'\r'));
    assert_eq!(carriage_returns, line_feeds);
}
