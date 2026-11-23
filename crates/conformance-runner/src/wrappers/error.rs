//! Failures of the parity check itself, as opposed to differences between forms.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// A failure that stops the parity check: a wrapper that cannot be run or answers outside its
/// protocol, or a document `core` cannot write (ARC-6.9).
#[derive(Debug)]
pub enum ParityError {
    /// A file or a process at `path` cannot be read, written or started.
    Io {
        /// The file or binary involved.
        path: PathBuf,
        /// The underlying error.
        error: io::Error,
    },
    /// `core` cannot write the document of a call of `case`.
    Output {
        /// The case of the call.
        case: String,
        /// Why.
        reason: String,
    },
    /// The MCP server answered a message that is not a JSON-RPC response (ARC-6.6).
    Protocol(String),
}

impl fmt::Display for ParityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, error } => {
                write!(formatter, "ARC-6.9: `{}`: {error}", path.display())
            }
            Self::Output { case, reason } => {
                write!(formatter, "ARC-6.9: case `{case}`: {reason}")
            }
            Self::Protocol(reason) => write!(formatter, "ARC-6.6: menura-mcp: {reason}"),
        }
    }
}

impl std::error::Error for ParityError {}

impl ParityError {
    /// An I/O error about `path`.
    pub(crate) fn io(path: impl Into<PathBuf>) -> impl FnOnce(io::Error) -> Self {
        let path = path.into();
        move |error| Self::Io { path, error }
    }
}
