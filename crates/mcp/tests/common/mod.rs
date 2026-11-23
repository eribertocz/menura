//! Helpers shared by the tests of `menura-mcp`.

#![allow(
    dead_code,
    clippy::disallowed_methods,
    reason = "each test file uses its own subset of the helpers; the tests read fixture files"
)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

pub const QUERY: &str = r#"{"contract": "menura.query", "contract_version": "1.1.0",
    "time_zone": "UTC", "query": {"dimensions": ["orders.region"]}}"#;
pub const CONTEXT: &str = r#"{"contract": "menura.security_context", "contract_version": "1.1.0",
    "subject": "user-1", "roles": ["admin"], "attributes": {}}"#;

/// The workspace root.
pub fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The path of a file of the core's shop fixture.
pub fn shop_path(file: &str) -> PathBuf {
    workspace()
        .join("crates/core/tests/fixtures/shop")
        .join(file)
}

/// The text of a file of the core's shop fixture.
pub fn shop(file: &str) -> String {
    fs::read_to_string(shop_path(file)).unwrap()
}

/// The answer of the server to `request`, parsed.
pub fn answer(request: &Value) -> Value {
    let line = menura_mcp::answer(&request.to_string()).unwrap();
    serde_json::from_str(&line).unwrap()
}

/// The answer to `tools/call` of `tool` with `arguments`.
pub fn call(tool: &str, arguments: &Value) -> Value {
    answer(&json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call",
        "params": {"name": tool, "arguments": arguments}}))
}

/// The arguments of `compile` and `explain` over the shop model.
pub fn shop_compile_arguments() -> Value {
    json!({"model": shop("model.yaml"), "catalog_schema": shop("catalog-schema.json"),
        "query": QUERY, "security_context": CONTEXT})
}

/// The `-32602` message of an answer, or a failure when it is not one.
pub fn usage_message(answer: &Value) -> String {
    assert_eq!(answer["error"]["code"], -32602, "{answer}");
    answer["error"]["message"].as_str().unwrap().to_owned()
}

/// The text content items and `isError` of a tool result.
pub fn texts(answer: &Value) -> (Vec<String>, bool) {
    let result = &answer["result"];
    let texts = result["content"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            assert_eq!(item["type"], "text");
            item["text"].as_str().unwrap().to_owned()
        })
        .collect();
    (texts, result["isError"].as_bool().unwrap())
}
