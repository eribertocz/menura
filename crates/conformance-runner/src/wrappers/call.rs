//! A call of one command, with its arguments, as every wrapper receives it (ARC-6.7, ARC-6.4).

use std::path::PathBuf;

use menura_core::{Argument, CommandName};

/// A document argument: its bytes, and the fixture file they come from, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// The fixture file, or `None` for a document the runner writes itself (a query).
    pub path: Option<PathBuf>,
    /// The bytes passed to `core` unchanged (ARC-5.10).
    pub bytes: Vec<u8>,
}

/// The value of one argument.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallValue {
    /// A document, passed as a file to the CLI and as text or bytes to the other forms.
    Document(Document),
    /// An options value, written as a JSON file for the CLI and passed as an object (MCP) or a
    /// `dict` (Python). It may be any JSON value, so that usage errors can be probed.
    Options(serde_json::Value),
}

/// One call of a command, made for a conformance case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapperCall {
    /// The case the call is made for, as shown in reports.
    pub case: String,
    /// The command, and its MCP tool.
    pub command: CommandName,
    /// The arguments, in the order the call gives them.
    pub arguments: Vec<(Argument, CallValue)>,
}

impl WrapperCall {
    /// The bytes of the document argument `argument`, if the call gives it.
    #[must_use]
    pub fn document(&self, argument: Argument) -> Option<&[u8]> {
        self.arguments.iter().find_map(|(name, value)| match value {
            CallValue::Document(document) if *name == argument => Some(document.bytes.as_slice()),
            _ => None,
        })
    }

    /// The options argument `argument`, if the call gives it.
    #[must_use]
    pub fn options(&self, argument: Argument) -> Option<&serde_json::Value> {
        self.arguments.iter().find_map(|(name, value)| match value {
            CallValue::Options(options) if *name == argument => Some(options),
            _ => None,
        })
    }

    /// Whether every document of the call is valid UTF-8, so that MCP can pass it as text
    /// (ARC-6.9).
    #[must_use]
    pub fn is_utf8(&self) -> bool {
        self.arguments.iter().all(|(_, value)| match value {
            CallValue::Document(document) => std::str::from_utf8(&document.bytes).is_ok(),
            CallValue::Options(_) => true,
        })
    }

    /// Whether the call compiles with `audit: true` (ARC-5.7).
    #[must_use]
    pub fn audits(&self) -> bool {
        self.command == CommandName::Compile
            && self
                .options(Argument::CompileOptions)
                .and_then(|options| options.get("audit"))
                .is_some_and(|audit| audit == &serde_json::Value::Bool(true))
    }
}
