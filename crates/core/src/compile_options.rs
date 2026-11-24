//! The compile options of architecture section 3.5, and their JSON object form (ARC-5.7).

use std::fmt;
use std::num::NonZeroU64;

use menura_contracts::types::common::Dialect;
use serde_json::Value;

use crate::dialect_name::{dialect_named, quoted_dialect_names};

/// The default of `limits.timeout_ms`, in milliseconds (architecture section 3.5).
const DEFAULT_TIMEOUT_MS: NonZeroU64 = match NonZeroU64::new(30_000) {
    Some(timeout) => timeout,
    None => NonZeroU64::MIN,
};

/// The options `compile` reads (architecture section 3.5).
///
/// # Example
///
/// ```
/// use menura_core::{CompileOptions, Dialect};
///
/// let options = CompileOptions::default();
/// assert_eq!(options.dialect, Dialect::Duckdb);
/// assert_eq!(options.timeout_ms.get(), 30000);
/// assert_eq!(options.max_rows, None);
/// assert!(!options.audit);
/// assert_eq!(options.actor, None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOptions {
    /// The target dialect, `clickhouse` or `duckdb`; `duckdb` by default.
    pub dialect: Dialect,
    /// Written to `limits.timeout_ms`; 30000 by default.
    pub timeout_ms: NonZeroU64,
    /// Written to `limits.max_rows`; `None` (no limit beyond the query's own) by default.
    pub max_rows: Option<NonZeroU64>,
    /// Also return the audit record (PLN-6), with the compiled plan or with the compile error;
    /// `false` by default.
    pub audit: bool,
    /// The delegated actor, authenticated by the host: invocation metadata written unchanged
    /// only in the audit record, outside every hash (PLN-6.8); absent by default.
    pub actor: Option<String>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            dialect: Dialect::Duckdb,
            timeout_ms: DEFAULT_TIMEOUT_MS,
            max_rows: None,
            audit: false,
            actor: None,
        }
    }
}

/// A compile-options document that is not a JSON object of compile options (ARC-5.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileOptionsError {
    message: String,
}

impl CompileOptionsError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CompileOptionsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CompileOptionsError {}

impl CompileOptions {
    /// Reads the compile options from `bytes`, a UTF-8 JSON object whose members are compile
    /// options; an option it omits takes its default (ARC-5.7, read as the load options are,
    /// ARC-5.5).
    ///
    /// # Errors
    ///
    /// A [`CompileOptionsError`] when `bytes` is not UTF-8 JSON, is not an object, has a member
    /// that is not a compile option, or has a value of the wrong type (`null` included):
    /// `dialect` is `clickhouse` or `duckdb`, `timeout_ms` and `max_rows` are positive integers, `audit` is a
    /// boolean and `actor` a string.
    ///
    /// # Example
    ///
    /// ```
    /// use menura_core::CompileOptions;
    ///
    /// let options = CompileOptions::from_json(br#"{"audit": true, "max_rows": 10}"#)?;
    /// assert!(options.audit);
    /// assert_eq!(options.max_rows.map(|rows| rows.get()), Some(10));
    /// assert!(CompileOptions::from_json(br#"{"max_rows": null}"#).is_err());
    /// # Ok::<(), menura_core::CompileOptionsError>(())
    /// ```
    pub fn from_json(bytes: &[u8]) -> Result<Self, CompileOptionsError> {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| CompileOptionsError::new("the compile options are not UTF-8"))?;
        let value: Value = serde_json::from_str(text).map_err(|error| {
            CompileOptionsError::new(format!("the compile options are not JSON: {error}"))
        })?;
        let Value::Object(members) = value else {
            return Err(CompileOptionsError::new(
                "the compile options are not a JSON object",
            ));
        };
        let mut options = Self::default();
        for (name, value) in &members {
            options.set(name, value)?;
        }
        Ok(options)
    }

    fn set(&mut self, name: &str, value: &Value) -> Result<(), CompileOptionsError> {
        match name {
            "dialect" => self.dialect = dialect(value)?,
            "timeout_ms" => self.timeout_ms = positive_integer(name, value)?,
            "max_rows" => self.max_rows = Some(positive_integer(name, value)?),
            "audit" => {
                self.audit = value.as_bool().ok_or_else(|| {
                    CompileOptionsError::new("the compile option `audit` must be a boolean")
                })?;
            }
            "actor" => {
                self.actor = Some(value.as_str().map(str::to_owned).ok_or_else(|| {
                    CompileOptionsError::new("the compile option `actor` must be a string")
                })?);
            }
            _ => {
                return Err(CompileOptionsError::new(format!(
                    "`{name}` is not a compile option; the options are `dialect`, `timeout_ms`, \
                     `max_rows`, `audit` and `actor`"
                )));
            }
        }
        Ok(())
    }
}

fn dialect(value: &Value) -> Result<Dialect, CompileOptionsError> {
    value.as_str().and_then(dialect_named).ok_or_else(|| {
        CompileOptionsError::new(format!(
            "the compile option `dialect` must be {}",
            quoted_dialect_names("or")
        ))
    })
}

fn positive_integer(name: &str, value: &Value) -> Result<NonZeroU64, CompileOptionsError> {
    value.as_u64().and_then(NonZeroU64::new).ok_or_else(|| {
        CompileOptionsError::new(format!(
            "the compile option `{name}` must be a positive integer"
        ))
    })
}
