//! The public function `compile` (architecture section 3.5, E2-5, PLN-1 to PLN-5).

use std::fmt;

use menura_contracts::types::compiled_plan::{CompileErrorDocument, CompiledPlanDocument};

use crate::compile_options::CompileOptions;
use crate::handle::Handle;

/// What `compile` returns: a compiled plan or a compile error (PLN-1, PLN-5), or an internal
/// error of the compiler.
#[derive(Debug, Clone, PartialEq)]
pub enum CompileResult {
    /// The compiled plan (PLN-1).
    Plan(Box<CompiledPlanDocument>),
    /// The compile error (PLN-5).
    Error(CompileErrorDocument),
    /// A stage received an input that the stage before it never produces: a defect of the
    /// compiler, not a query error, so it has no specification code.
    Internal(InternalError),
}

/// A defect of the compiler: the stage that found it and what it found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InternalError {
    /// The stage that found the defect, such as `E3`.
    pub stage: &'static str,
    /// What the stage found.
    pub message: String,
}

impl fmt::Display for InternalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "internal error in {}: {}",
            self.stage, self.message
        )
    }
}

impl std::error::Error for InternalError {}

/// Compiles the query document `query` for the user of the security context document
/// `security_context`, over the model of `handle`, with `options`.
#[must_use]
pub fn compile(
    handle: &Handle,
    query: &[u8],
    security_context: &[u8],
    options: &CompileOptions,
) -> CompileResult {
    let _ = (handle, query, security_context, options);
    CompileResult::Internal(InternalError {
        stage: "core",
        message: "not implemented".to_owned(),
    })
}
