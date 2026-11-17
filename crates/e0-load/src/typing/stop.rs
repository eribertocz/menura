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
    /// A case the specification does not define yet (issue #126). The element gets no type and
    /// no typing diagnostic until the issue is settled.
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
