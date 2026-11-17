//! Why typing an expression stops before it finds a type.

use super::diagnostic::TypingCode;

/// The outcome of typing an expression or a part of it.
pub(crate) type Outcome<T> = Result<T, Stop>;

/// Why an expression has no type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Stop {
    /// A typing problem: the element is unusable with this code.
    Problem {
        /// The code.
        code: TypingCode,
        /// What the problem is, in English.
        message: String,
    },
    /// The expression reads something another check reports: a missing or unusable reference
    /// (E0.d, or a field this substage already reports), `AGG(metric)` (E0C-37 and the derived
    /// and context metrics) or `USER_ATTRIBUTE` (row policies). The element gets no type and no
    /// typing diagnostic.
    ReportedElsewhere,
    /// A case the specification does not define yet, raised with the maintainer on pull request
    /// #131: a simple `CASE` comparing numeric values of different kinds, and an expression that
    /// is the bare literal `NULL`. The element gets no type and no typing diagnostic until it is
    /// settled.
    Undecided,
}

/// A typing problem with `code`.
pub(crate) fn problem<T>(code: TypingCode, message: String) -> Outcome<T> {
    Err(Stop::Problem { code, message })
}

/// `E0F-03` with `message`.
pub(crate) fn incompatible<T>(message: String) -> Outcome<T> {
    problem(TypingCode::IncompatibleTypes, message)
}

/// `E0F-03` for a literal `NULL` that no other operand gives a type (E0F-3.2).
pub(crate) fn untyped_null<T>(role: &str) -> Outcome<T> {
    incompatible(format!(
        "{role} is the literal NULL, and no other operand gives it a type; a CAST gives it one"
    ))
}
