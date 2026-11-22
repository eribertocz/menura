//! The compile options of architecture section 3.5.

use std::num::NonZeroU64;

use menura_contracts::types::common::Dialect;

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
    /// The target dialect; `duckdb` by default.
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
