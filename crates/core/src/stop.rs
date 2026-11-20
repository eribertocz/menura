//! Why compilation stopped, and the diagnostics `compile` writes itself.

use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_e2_bind::BindError;
use menura_e4_plan::PlanError;

use crate::compile::InternalError;

/// Why compilation stopped: the first error of E2-5, or a defect of the compiler.
pub(crate) enum Stop {
    /// The first error, in the order of E2-5 (PLN-5.1).
    Error(Diagnostic),
    /// A defect of the compiler.
    Internal(InternalError),
}

impl Stop {
    /// The error of the compile error document: the first error, or `E_INTERNAL` without
    /// location, naming the stage, for a defect of the compiler (PLN-5.1a).
    pub(crate) fn into_diagnostic(self) -> Diagnostic {
        match self {
            Self::Error(diagnostic) => diagnostic,
            Self::Internal(error) => Diagnostic {
                code: "E_INTERNAL".to_owned(),
                message: format!(
                    "internal error of the compiler in stage {}: {}; report it with the inputs \
                     that produced it",
                    error.stage, error.message
                ),
                location: None,
            },
        }
    }
}

impl From<Diagnostic> for Stop {
    fn from(diagnostic: Diagnostic) -> Self {
        Self::Error(diagnostic)
    }
}

impl From<InternalError> for Stop {
    fn from(error: InternalError) -> Self {
        Self::Internal(error)
    }
}

/// The stop of an E2 error.
pub(crate) fn bind_stop(error: BindError) -> Stop {
    match error {
        BindError::Diagnostic(diagnostic) => diagnostic.to_diagnostic().into(),
        BindError::Invariant {
            violation,
            location,
        } => InternalError::new("E2", format!("{violation:?} at {}", location.pointer())).into(),
    }
}

/// The stop of an E4 error.
pub(crate) fn plan_stop(error: &PlanError) -> Stop {
    match error.to_diagnostic() {
        Some(diagnostic) => diagnostic.into(),
        None => InternalError::new("E4", error.to_string()).into(),
    }
}

/// `E_NOT_YET_SUPPORTED` for `audit: true`, which comes in 0.7.0 (VS-5).
pub(crate) fn audit_not_yet_supported() -> Diagnostic {
    Diagnostic {
        code: "E_NOT_YET_SUPPORTED".to_owned(),
        message: "the audit record (`audit: true`) is not supported before 0.7.0; compile \
                  without `audit`"
            .to_owned(),
        location: None,
    }
}

/// A security context that is not UTF-8 is not JSON: `ESEC-01` at the empty pointer
/// (SEC-3.8).
pub(crate) fn context_not_utf8() -> Stop {
    not_utf8(
        "ESEC-01",
        InputDocument::SecurityContext,
        "security context",
    )
}

/// A query that is not UTF-8 is not JSON: `E_INVALID_QUERY` at the empty pointer (E1-2.1).
pub(crate) fn query_not_utf8() -> Stop {
    not_utf8("E_INVALID_QUERY", InputDocument::Query, "query")
}

fn not_utf8(code: &str, document: InputDocument, name: &str) -> Stop {
    Diagnostic {
        code: code.to_owned(),
        message: format!("the {name} is not valid UTF-8, so it is not JSON; write it as UTF-8"),
        location: Some(DiagnosticLocation {
            document,
            pointer: String::new(),
            line: None,
            column: None,
        }),
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::Stop;
    use crate::compile::InternalError;

    #[test]
    fn pln_5_1a_a_defect_of_the_compiler_is_e_internal_without_location() {
        let diagnostic =
            Stop::Internal(InternalError::new("E6", "unknown column")).into_diagnostic();
        assert_eq!(diagnostic.code, "E_INTERNAL");
        assert_eq!(diagnostic.location, None);
        assert!(diagnostic.message.contains("stage E6"));
        assert!(diagnostic.message.contains("unknown column"));
    }
}
