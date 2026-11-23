//! `menura-mcp` transport, session, tools and results: ARC-6.1, ARC-6.6 to ARC-6.8.

#![cfg(test)]
#![expect(
    clippy::disallowed_types,
    reason = "ARC-6.1: the test runs the server as a process over standard input and output"
)]

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use common::{CONTEXT, answer, call, shop, shop_compile_arguments, texts, usage_message};
use serde_json::{Value, json};

const TOOLS: [&str; 8] = [
    "validate",
    "compile",
    "explain",
    "describe",
    "integrity_checks",
    "dependencies",
    "migrate",
    "diff",
];

#[test]
fn arc_6_6_initialize_pins_the_revision_whatever_the_client_asks() {
    let answer = answer(&json!({"jsonrpc": "2.0", "id": "a", "method": "initialize",
        "params": {"protocolVersion": "2024-11-05", "capabilities": {},
            "clientInfo": {"name": "test", "version": "1"}}}));
    assert_eq!(answer["id"], "a");
    let result = &answer["result"];
    assert_eq!(result["protocolVersion"], "2025-06-18");
    assert_eq!(
        result["serverInfo"],
        json!({"name": "menura-mcp", "version": env!("CARGO_PKG_VERSION")})
    );
    assert_eq!(
        result["capabilities"],
        json!({"tools": {"listChanged": false}})
    );
}

#[test]
fn arc_6_6_a_call_before_initialize_is_answered_as_after() {
    let first = call("migrate", &json!({"model": shop("model.yaml")}));
    answer(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}));
    let second = call("migrate", &json!({"model": shop("model.yaml")}));
    assert_eq!(first, second);
}

#[test]
fn arc_6_6_notifications_are_not_answered() {
    for method in ["notifications/initialized", "notifications/cancelled"] {
        let message = json!({"jsonrpc": "2.0", "method": method, "params": {"requestId": 1}});
        assert_eq!(menura_mcp::answer(&message.to_string()), None);
    }
}

#[test]
fn arc_6_6_the_binary_writes_only_protocol_messages_on_standard_output() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_menura-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "migrate", "arguments": {"model": shop("model.yaml")}}}),
    ];
    let mut stdin = child.stdin.take().unwrap();
    for message in &input {
        writeln!(stdin, "{message}").unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let lines: Vec<Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let ids: Vec<&Value> = lines.iter().map(|line| &line["id"]).collect();
    assert_eq!(ids, [&json!(1), &json!(2), &json!(3)]);
    assert!(lines.iter().all(|line| line["jsonrpc"] == "2.0"));
}

#[test]
fn arc_6_6_a_message_that_is_not_a_request_is_a_json_rpc_error() {
    let parse = menura_mcp::answer("{").unwrap();
    assert!(parse.contains("-32700"), "{parse}");
    let unknown = answer(&json!({"jsonrpc": "2.0", "id": 4, "method": "resources/list"}));
    assert_eq!(unknown["error"]["code"], -32601);
}

/// The tool names of a `tools/list` answer, in order.
fn tool_names(list: &Value) -> Vec<&str> {
    list["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect()
}

#[test]
fn arc_6_7_there_is_one_tool_per_cli_command() {
    let list = answer(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    assert_eq!(tool_names(&list), TOOLS);
}

#[test]
fn arc_6_1_the_binary_lists_one_tool_per_cli_command_over_standard_input_and_output() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_menura-mcp"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(
        stdin,
        "{}",
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})
    )
    .unwrap();
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 1, "{stdout}");
    let list: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(tool_names(&list), TOOLS);
}

#[test]
fn arc_6_7_input_schemas_are_closed_objects_of_the_command_flags() {
    let list = answer(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let compile = tools.iter().find(|tool| tool["name"] == "compile").unwrap();
    let schema = &compile["inputSchema"];
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);
    let members: Vec<&String> = schema["properties"].as_object().unwrap().keys().collect();
    assert_eq!(
        members,
        [
            "catalog_schema",
            "catalog_state",
            "compile_options",
            "load_options",
            "model",
            "query",
            "security_context"
        ]
    );
    assert_eq!(schema["properties"]["model"]["type"], "string");
    assert_eq!(schema["properties"]["compile_options"]["type"], "object");
    assert_eq!(
        schema["required"],
        json!(["model", "catalog_schema", "query", "security_context"])
    );
}

#[test]
fn arc_6_7_usage_errors_are_minus_32602_with_member_names() {
    let shop_model = shop("model.yaml");
    let cases = [
        ("run", json!({}), "unknown tool `run`"),
        (
            "validate",
            json!({"model": shop_model}),
            "`catalog_schema` is required",
        ),
        (
            "validate",
            json!({"model": shop_model, "catalog_schema": "{}", "model_name": "x"}),
            "unknown argument `model_name`",
        ),
        (
            "explain",
            json!({"model": shop_model, "catalog_schema": "{}", "query": "{}",
                "security_context": "{}", "audit_record": "r.json"}),
            "unknown argument `audit_record`",
        ),
        ("migrate", json!({"model": null}), "`model` is not a string"),
        (
            "integrity_checks",
            json!({"model": shop_model, "catalog_schema": "{}", "dialect": "postgres"}),
            "unknown dialect `postgres`; the dialect is `duckdb`",
        ),
    ];
    for (tool, arguments, message) in cases {
        assert_eq!(
            usage_message(&call(tool, &arguments)),
            message,
            "{tool} {arguments}"
        );
    }
}

#[test]
fn arc_6_7_invalid_options_give_the_cli_reason_without_a_path() {
    let arguments = json!({"model": shop("model.yaml"), "load_options": {"namespace": null}});
    let message = usage_message(&call("migrate", &arguments));
    assert!(message.starts_with("`load_options`: "), "{message}");
    let arguments = json!({"model": shop("model.yaml"), "load_options": null});
    let message = usage_message(&call("migrate", &arguments));
    assert!(message.starts_with("`load_options`: "), "{message}");
}

#[test]
fn arc_6_7_compile_with_audit_needs_no_audit_record_member() {
    let mut arguments = shop_compile_arguments();
    arguments["compile_options"] = json!({"audit": true});
    let (texts, is_error) = texts(&call("compile", &arguments));
    assert!(!is_error);
    assert_eq!(texts.len(), 2);
}

#[test]
fn arc_6_8_a_tool_returns_one_text_item_and_is_error_as_exit_1() {
    let (plan, is_error) = texts(&call("compile", &shop_compile_arguments()));
    assert!(!is_error);
    assert_eq!(plan.len(), 1);
    assert!(!plan[0].ends_with('\n'));
    let document: Value = serde_json::from_str(&plan[0]).unwrap();
    assert_eq!(document["contract"], "menura.compiled_plan");

    let mut arguments = shop_compile_arguments();
    arguments["query"] = json!("{}");
    let (error, is_error) = texts(&call("compile", &arguments));
    assert!(is_error);
    let document: Value = serde_json::from_str(&error[0]).unwrap();
    assert_eq!(document["contract"], "menura.compile_error");
}

#[test]
fn arc_6_8_the_audit_record_follows_the_plan_or_the_compile_error() {
    let mut arguments = shop_compile_arguments();
    arguments["compile_options"] = json!({"audit": true});
    arguments["query"] = json!("{}");
    let (items, is_error) = texts(&call("compile", &arguments));
    assert!(is_error);
    assert_eq!(items.len(), 2);
    let record: Value = serde_json::from_str(&items[1]).unwrap();
    assert_eq!(record["contract"], "menura.audit_record");
}

#[test]
fn arc_6_8_a_model_that_does_not_load_gives_no_audit_record() {
    let arguments = json!({"model": "version: \"0.1.1\"", "catalog_schema": "{}",
        "query": "{}", "security_context": CONTEXT, "compile_options": {"audit": true}});
    let (items, is_error) = texts(&call("compile", &arguments));
    assert!(is_error);
    assert_eq!(items.len(), 1);
}

#[test]
fn arc_6_8_no_structured_content_or_output_schema_is_used() {
    let list = answer(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}));
    for tool in list["result"]["tools"].as_array().unwrap() {
        assert!(tool.get("outputSchema").is_none());
    }
    let result = &call("compile", &shop_compile_arguments())["result"];
    assert!(result.get("structuredContent").is_none());
}
