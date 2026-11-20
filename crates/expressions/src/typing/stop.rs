//! Why typing an expression stops before it finds a type.

use super::code::TypingCode;

/// The outcome of typing an expression or a part of it.
pub type Outcome<T> = Result<T, Stop>;

/// Why an expression has no type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stop {
    /// A typing problem with this code (E0F-8).
    Problem {
        /// The code.
        code: TypingCode,
        /// What the problem is, in English.
        message: String,
    },
    /// The expression reads something the scope leaves to another check: a reference the
    /// resolver gives no type (in E0, one that E0.d reports), `AGG(metric)` where
    /// [`MetricAggregates::ReportedElsewhere`] applies (in E0, `E0C-37` and the derived and
    /// context metrics) or `USER_ATTRIBUTE` without an attribute resolver (in E0, row
    /// policies). The expression gets no type and no typing diagnostic.
    ///
    /// [`MetricAggregates::ReportedElsewhere`]: super::MetricAggregates::ReportedElsewhere
    ReportedElsewhere,
}

/// A typing problem with `code`.
///
/// # Errors
///
/// Always: [`Stop::Problem`] with `code` and `message`.
pub fn problem<T>(code: TypingCode, message: String) -> Outcome<T> {
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
