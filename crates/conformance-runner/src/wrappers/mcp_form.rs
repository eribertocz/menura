//! The MCP form: calls sent as `tools/call` requests to one `menura-mcp` session, the text
//! content of each result compared with `core` (ARC-6.6 to ARC-6.9).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Map, Value, json};

use super::call::{CallValue, WrapperCall};
use super::difference::{Mismatch, compare_text};
use super::error::ParityError;
use super::expected::{Expected, ExpectedOutput};

/// The JSON-RPC error code of refused arguments (ARC-6.7).
const INVALID_PARAMS: i64 = -32602;

/// One running `menura-mcp` session.
#[derive(Debug)]
pub struct McpForm {
    binary: PathBuf,
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpForm {
    /// Starts the server at `binary` and initializes the session (ARC-6.6).
    ///
    /// # Errors
    ///
    /// Returns [`ParityError`] when the server cannot start or does not answer `initialize`.
    pub fn start(binary: &Path) -> Result<Self, ParityError> {
        let mut child = Command::new(binary)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(ParityError::io(binary))?;
        let (Some(input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
            return Err(ParityError::Protocol("no standard streams".to_owned()));
        };
        let mut form = Self {
            binary: binary.to_path_buf(),
            child,
            input,
            output: BufReader::new(output),
            next_id: 0,
        };
        form.request(
            "initialize",
            &json!({"protocolVersion": "2025-06-18",
            "capabilities": {}, "clientInfo": {"name": "wrapper-parity", "version": "1"}}),
        )?;
        form.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}))?;
        Ok(form)
    }

    /// Whether `call` can be sent: every document is valid UTF-8 (ARC-6.9).
    #[must_use]
    pub fn accepts(call: &WrapperCall) -> bool {
        call.is_utf8()
    }

    /// Sends `call` as a tool call and compares its result with `expected`. A call that
    /// [`McpForm::accepts`] refuses must not be sent.
    ///
    /// # Errors
    ///
    /// Returns [`ParityError`] when the session fails or answers outside JSON-RPC.
    pub fn check(
        &mut self,
        call: &WrapperCall,
        expected: &Expected,
    ) -> Result<Option<Mismatch>, ParityError> {
        let mut arguments = Map::new();
        for (argument, value) in &call.arguments {
            let value = match value {
                CallValue::Document(document) => {
                    Value::String(String::from_utf8_lossy(&document.bytes).into_owned())
                }
                CallValue::Options(options) => options.clone(),
            };
            arguments.insert(argument.name().to_owned(), value);
        }
        let params = json!({"name": call.command.name(), "arguments": arguments});
        let response = self.request("tools/call", &params)?;
        let detail = match expected {
            Expected::Output(expected) => output_difference(expected, &response),
            Expected::Usage(error) => usage_difference(&error.to_string(), &response),
        };
        Ok(detail.map(|detail| Mismatch {
            case: call.case.clone(),
            wrapper: "mcp",
            command: call.command.name(),
            detail,
        }))
    }

    /// Sends a request and reads its response.
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, ParityError> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        let mut line = String::new();
        self.output
            .read_line(&mut line)
            .map_err(ParityError::io(&self.binary))?;
        let response: Value = serde_json::from_str(&line)
            .map_err(|error| ParityError::Protocol(format!("`{}`: {error}", line.trim_end())))?;
        if response.get("id") != Some(&json!(id)) {
            return Err(ParityError::Protocol(format!(
                "the response to request {id} is `{}`",
                line.trim_end()
            )));
        }
        Ok(response)
    }

    /// Writes one message on its own line.
    fn send(&mut self, message: &Value) -> Result<(), ParityError> {
        writeln!(self.input, "{message}")
            .and_then(|()| self.input.flush())
            .map_err(ParityError::io(&self.binary))
    }
}

impl Drop for McpForm {
    fn drop(&mut self) {
        // The session ends with the parity check; a server that already exited is not an error.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// What differs between a tool result and `expected`, if anything (ARC-6.8).
fn output_difference(expected: &ExpectedOutput, response: &Value) -> Option<String> {
    let Some(result) = response.get("result") else {
        return Some(format!("the call was refused: {}", response["error"]));
    };
    let is_error = result
        .get("isError")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if is_error != expected.is_error {
        return Some(format!(
            "isError {is_error}, expected {}",
            expected.is_error
        ));
    }
    let texts: Vec<&str> = result
        .get("content")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item["text"].as_str())
                .collect()
        })
        .unwrap_or_default();
    let wanted = 1 + usize::from(expected.audit_record.is_some());
    if texts.len() != wanted {
        return Some(format!("{} text items, expected {wanted}", texts.len()));
    }
    compare_text(
        "text content",
        &expected.document,
        texts.first().unwrap_or(&""),
    )
    .or_else(|| {
        expected
            .audit_record
            .as_ref()
            .and_then(|record| compare_text("audit record", record, texts.get(1).unwrap_or(&"")))
    })
}

/// What differs between a refused call and the usage error `message` (ARC-6.7).
fn usage_difference(message: &str, response: &Value) -> Option<String> {
    let error = response.get("error");
    let code = error.and_then(|error| error["code"].as_i64());
    if code != Some(INVALID_PARAMS) {
        return Some(format!(
            "answer {response}, expected the error {INVALID_PARAMS}"
        ));
    }
    let found = error
        .and_then(|error| error["message"].as_str())
        .unwrap_or("");
    compare_text("error message", message, found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expected() -> ExpectedOutput {
        ExpectedOutput {
            document: "{\n  \"sql\": \"SELECT 1\"\n}".to_owned(),
            is_error: false,
            audit_record: None,
        }
    }

    fn response(text: &str) -> Value {
        json!({"jsonrpc": "2.0", "id": 1, "result": {"content": [{"type": "text", "text": text}]}})
    }

    #[test]
    fn arc_6_9_an_injected_difference_in_a_tool_result_is_reported() {
        let detail = output_difference(&expected(), &response("{\n  \"sql\": \"SELECT 2\"\n}"));
        assert_eq!(
            detail.unwrap(),
            "text content: line 2, column 18: expected `  \"sql\": \"SELECT 1\"`, \
             found `  \"sql\": \"SELECT 2\"`"
        );
    }

    #[test]
    fn arc_6_9_an_equal_tool_result_has_no_difference() {
        assert_eq!(
            output_difference(&expected(), &response(&expected().document)),
            None
        );
    }
}
