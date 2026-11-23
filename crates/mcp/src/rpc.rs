//! JSON-RPC 2.0 messages of MCP revision `2025-06-18`: one message per line, each answered on
//! its own, whatever came before (ARC-6.6).

use menura_core::{Argument, UsageError, UsageReason};
use serde_json::{Map, Value, json};

use crate::arguments::parse_call;
use crate::call::{CallFailure, ToolOutput, run_call};
use crate::tools::tool_list;

/// The MCP revision the server speaks, pinned per release (ARC-6.6).
pub(crate) const PROTOCOL_VERSION: &str = "2025-06-18";

/// The JSON-RPC error code of a message that is not JSON.
const PARSE_ERROR: i64 = -32700;
/// The JSON-RPC error code of a message that is not a request.
const INVALID_REQUEST: i64 = -32600;
/// The JSON-RPC error code of an unknown method.
const METHOD_NOT_FOUND: i64 = -32601;
/// The JSON-RPC error code of refused arguments (ARC-6.7).
const INVALID_PARAMS: i64 = -32602;
/// The JSON-RPC error code of a document that could not be written.
const INTERNAL_ERROR: i64 = -32603;

/// The answer to `line`, one JSON-RPC message, or `None` for a notification or a response.
pub(crate) fn answer(line: &str) -> Option<String> {
    let response = match serde_json::from_str::<Value>(line) {
        Ok(Value::Object(message)) => answer_message(&message)?,
        Ok(_) => error_response(&Value::Null, INVALID_REQUEST, "Invalid Request"),
        Err(_) => error_response(&Value::Null, PARSE_ERROR, "Parse error"),
    };
    Some(response.to_string())
}

/// The answer to `message`, a JSON object.
fn answer_message(message: &Map<String, Value>) -> Option<Value> {
    let id = message.get("id")?;
    if !matches!(id, Value::String(_) | Value::Number(_)) {
        return Some(error_response(
            &Value::Null,
            INVALID_REQUEST,
            "Invalid Request",
        ));
    }
    let method = message.get("method").and_then(Value::as_str);
    let Some(method) = method.filter(|_| message.get("jsonrpc") == Some(&json!("2.0"))) else {
        if message.contains_key("result") || message.contains_key("error") {
            return None;
        }
        return Some(error_response(id, INVALID_REQUEST, "Invalid Request"));
    };
    Some(match method {
        "initialize" => result_response(id, &initialize_result()),
        "ping" => result_response(id, &json!({})),
        "tools/list" => result_response(id, &tool_list()),
        "tools/call" => call_tool(id, message.get("params")),
        _ => error_response(id, METHOD_NOT_FOUND, "Method not found"),
    })
}

/// The result of `initialize`, whatever revision the client asks for (ARC-6.6).
fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "serverInfo": {"name": "menura-mcp", "version": env!("CARGO_PKG_VERSION")},
        "capabilities": {"tools": {"listChanged": false}},
    })
}

/// The answer to `tools/call` with `params` (ARC-6.7, ARC-6.8).
fn call_tool(id: &Value, params: Option<&Value>) -> Value {
    let name = params
        .and_then(|params| params.get("name"))
        .and_then(Value::as_str);
    let Some(name) = name else {
        return error_response(id, INVALID_PARAMS, "the tool call has no `name` string");
    };
    let arguments = params.and_then(|params| params.get("arguments"));
    let outcome = parse_call(name, arguments)
        .map_err(CallFailure::Usage)
        .and_then(|call| run_call(&call));
    match outcome {
        Ok(output) => result_response(id, &tool_result(output)),
        Err(CallFailure::Usage(error)) => {
            error_response(id, INVALID_PARAMS, &usage_message(&error))
        }
        Err(CallFailure::Output(error)) => error_response(id, INTERNAL_ERROR, &error.to_string()),
    }
}

/// The message of a usage error, with MCP member names; an unknown tool's message is
/// ``unknown tool `<name>` `` (ARC-6.7).
pub(crate) fn usage_message(error: &UsageError) -> String {
    match &error.reason {
        UsageReason::UnknownCommand(name) => format!("unknown tool `{name}`"),
        _ => error.message(|argument: Argument| argument.name().to_owned()),
    }
}

/// The result of a tool: its text content items and `isError` (ARC-6.8).
fn tool_result(output: ToolOutput) -> Value {
    let content: Vec<Value> = output
        .texts
        .into_iter()
        .map(|text| json!({"type": "text", "text": text}))
        .collect();
    json!({"content": content, "isError": output.is_error})
}

fn result_response(id: &Value, result: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error_response(id: &Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}
