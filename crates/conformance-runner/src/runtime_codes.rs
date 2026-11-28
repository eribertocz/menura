//! Maps an engine error to the runtime code of Menura it carries (EXE-5.3), the way an executor
//! of a compiled plan must: by the prefix of the raised message alone.
//!
//! A runtime failure of a plan is raised by its SQL with a constant message that starts with the
//! runtime code, a colon and a space (EXE-5.1). The engine wraps that raised message in its own
//! error (EXE-5.2): `DuckDB` writes `Invalid Input Error: ` before it, and `ClickHouse` reports
//! error 395 with its own text before and after it.

use std::fmt;

/// The text `DuckDB` writes before the raised message of `error(…)` (EXE-5.2).
const DUCKDB_RAISED_PREFIX: &str = "Invalid Input Error: ";

/// The start of the `ClickHouse` error that `throwIf` raises, error 395
/// (`FUNCTION_THROW_IF_VALUE_IS_NON_ZERO`, EXE-5.2).
const CLICKHOUSE_THROW_IF_START: &str = "Code: 395.";

/// The text `ClickHouse` writes right before the raised message of `throwIf`.
const CLICKHOUSE_RAISED_PREFIX: &str = "DB::Exception: ";

/// The separator between the runtime code and the rest of the raised message (EXE-5.1).
const CODE_SEPARATOR: &str = ": ";

/// The engine that reported an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// `DuckDB`, which raises the message with `error(…)` (E6-4.6a, E6-4.6b).
    Duckdb,
    /// `ClickHouse`, which raises the message with `throwIf` (E6-7.5).
    Clickhouse,
}

/// A runtime code of Menura: `R`, upper-case letters, `-` and two digits, such as `RDER-01`
/// (EXE-5.3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeCode(String);

impl RuntimeCode {
    /// The code `text`, or `None` when it does not have the form of a runtime code.
    ///
    /// # Example
    ///
    /// ```
    /// use menura_conformance_runner::runtime_codes::RuntimeCode;
    ///
    /// assert!(RuntimeCode::parse("RADD-02").is_some());
    /// assert!(RuntimeCode::parse("E6-01").is_none());
    /// ```
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let (letters, number) = text.split_once('-')?;
        let family = letters.strip_prefix('R')?;
        let family_valid =
            !family.is_empty() && family.bytes().all(|byte| byte.is_ascii_uppercase());
        let number_valid = number.len() == 2 && number.bytes().all(|byte| byte.is_ascii_digit());
        (family_valid && number_valid).then(|| Self(text.to_owned()))
    }

    /// The code as text, such as `RDER-01`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RuntimeCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The runtime code that the error `error_text` of `engine` carries, or `None` when it carries
/// none (EXE-5.3).
///
/// The code is read from the raised message only: the message `DuckDB` reports after
/// `Invalid Input Error: `, or the message `ClickHouse` reports after `DB::Exception: ` in its
/// error 395. It is the text before the first `: ` of that message, and only when it has the
/// form of a [`RuntimeCode`]. The rest of the message is never read, and an error of any other
/// kind, such as an overflow or a failed conversion, carries no code.
///
/// # Example
///
/// ```
/// use menura_conformance_runner::runtime_codes::{Engine, runtime_code};
///
/// let code = runtime_code(
///     Engine::Duckdb,
///     "Invalid Input Error: RDER-01: division by zero in metric margin",
/// );
/// assert_eq!(code.map(|code| code.to_string()), Some("RDER-01".to_owned()));
/// assert!(runtime_code(Engine::Duckdb, "Out of Range Error: Overflow").is_none());
/// ```
#[must_use]
pub fn runtime_code(engine: Engine, error_text: &str) -> Option<RuntimeCode> {
    let raised = match engine {
        Engine::Duckdb => error_text.strip_prefix(DUCKDB_RAISED_PREFIX)?,
        Engine::Clickhouse => clickhouse_raised_message(error_text)?,
    };
    let (prefix, _) = raised.split_once(CODE_SEPARATOR)?;
    RuntimeCode::parse(prefix)
}

/// The raised message inside a `ClickHouse` error 395, or `None` for any other error.
fn clickhouse_raised_message(error_text: &str) -> Option<&str> {
    let rest = error_text
        .trim_start()
        .strip_prefix(CLICKHOUSE_THROW_IF_START)?;
    let start = rest.find(CLICKHOUSE_RAISED_PREFIX)? + CLICKHOUSE_RAISED_PREFIX.len();
    rest.get(start..)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(engine: Engine, text: &str) -> Option<String> {
        runtime_code(engine, text).map(|code| code.as_str().to_owned())
    }

    #[test]
    fn exe_5_3_maps_a_duckdb_error_message_by_its_prefix() {
        assert_eq!(
            code(
                Engine::Duckdb,
                "Invalid Input Error: RDER-01: division by zero in metric margin"
            ),
            Some("RDER-01".to_owned())
        );
        assert_eq!(
            code(
                Engine::Duckdb,
                "Invalid Input Error: RADD-04: a period has no row in metric stock"
            ),
            Some("RADD-04".to_owned())
        );
    }

    #[test]
    fn exe_5_3_maps_a_clickhouse_throw_if_message_by_its_prefix() {
        let text = "Code: 395. DB::Exception: RADD-02: a value is NULL in metric stock: while \
                    executing 'FUNCTION throwIf(greater(x, 0) :: 3, 'RADD-02: a value is NULL in \
                    metric stock' :: 1) -> throwIf(...) UInt8 : 2'. \
                    (FUNCTION_THROW_IF_VALUE_IS_NON_ZERO) (version 25.8.1.1 (official build))\n";
        assert_eq!(code(Engine::Clickhouse, text), Some("RADD-02".to_owned()));
    }

    #[test]
    fn exe_5_3_a_message_without_a_code_carries_none() {
        for text in [
            "Out of Range Error: Overflow in multiplication of INT64",
            "Conversion Error: Could not convert string 'x' to INT32",
            "Invalid Input Error: division by zero",
            "Invalid Input Error: rder-01: lower case",
            "Invalid Input Error: RDER-1: one digit",
            "Invalid Input Error: E6-01: a compile-time code",
            "Invalid Input Error: RDER-01 without the separator",
        ] {
            assert_eq!(code(Engine::Duckdb, text), None, "{text}");
        }
        assert_eq!(
            code(
                Engine::Clickhouse,
                "Code: 407. DB::Exception: RDER-01: Decimal math overflow. (DECIMAL_OVERFLOW)"
            ),
            None
        );
        assert_eq!(
            code(
                Engine::Clickhouse,
                "Code: 395. DB::Exception: Value passed to 'throwIf' function is non-zero. \
                 (FUNCTION_THROW_IF_VALUE_IS_NON_ZERO)"
            ),
            None
        );
    }

    #[test]
    fn exe_5_3_never_takes_a_code_from_elsewhere_in_the_error() {
        assert_eq!(
            code(
                Engine::Duckdb,
                "Binder Error: column RDER-01: not found, Invalid Input Error: RADD-01: x"
            ),
            None
        );
        assert_eq!(
            code(Engine::Duckdb, "Invalid Input Error: overflow: RDER-01: x"),
            None
        );
        assert_eq!(
            code(
                Engine::Clickhouse,
                "Code: 395. DB::Exception: condition holds: RADD-01: x. \
                 (FUNCTION_THROW_IF_VALUE_IS_NON_ZERO)"
            ),
            None
        );
    }

    #[test]
    fn exe_5_3_a_runtime_code_has_the_form_r_letters_dash_two_digits() {
        for valid in ["RDER-01", "RADD-04", "REXE-03", "RX-99"] {
            assert!(RuntimeCode::parse(valid).is_some(), "{valid}");
        }
        for invalid in ["R-01", "DER-01", "RDER-001", "RDer-01", "RDER-0a", "RDER01"] {
            assert!(RuntimeCode::parse(invalid).is_none(), "{invalid}");
        }
    }
}
