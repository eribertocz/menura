#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod arguments;
mod call;
mod rpc;
mod tools;

/// The answer to `line`, one JSON-RPC message of MCP revision `2025-06-18`, as one line of
/// JSON; `None` for a notification or a response, which are not answered (ARC-6.6).
///
/// Every message is answered on its own: the answer never depends on earlier messages.
///
/// ```
/// let answer = menura_mcp::answer(r#"{"jsonrpc": "2.0", "id": 1, "method": "ping"}"#);
/// assert_eq!(answer.as_deref(), Some(r#"{"id":1,"jsonrpc":"2.0","result":{}}"#));
/// assert_eq!(menura_mcp::answer(r#"{"jsonrpc": "2.0", "method": "notifications/initialized"}"#), None);
/// ```
#[must_use]
pub fn answer(line: &str) -> Option<String> {
    rpc::answer(line)
}

#[cfg(test)]
mod documentation_tests;
