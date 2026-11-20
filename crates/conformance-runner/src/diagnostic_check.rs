//! Compares the diagnostics of a case with its expectation: the expected warnings,
//! and the location of the first error (CS-2.9).

use menura_core::{Diagnostic, DiagnosticLocation, InputDocument};

use crate::case::ExpectedLocation;

/// Why the reported warnings lack an expected code, or `None` when every expected code is
/// reported, each by its own warning.
pub(crate) fn missing_warnings<'report>(
    expected: &[String],
    reported: impl Iterator<Item = &'report Diagnostic>,
) -> Option<String> {
    let mut remaining: Vec<&str> = reported.map(|warning| warning.code.as_str()).collect();
    let reported = remaining.join(", ");
    for code in expected {
        let Some(index) = remaining.iter().position(|reported| reported == code) else {
            return Some(format!(
                "the load warning {code} is not reported; the warnings are [{reported}]"
            ));
        };
        remaining.remove(index);
    }
    None
}

/// Why the location of the first error differs from `expected`, or `None` when it matches:
/// `document` and `pointer` exactly, and `line` and `column` when the case gives them (CS-2.9).
pub(crate) fn location_mismatch(
    expected: &ExpectedLocation,
    reported: Option<&DiagnosticLocation>,
) -> Option<String> {
    let Some(reported) = reported else {
        return Some("the first load error has no location, but the case expects one".to_owned());
    };
    let matches = expected.document == reported.document
        && expected.pointer == reported.pointer
        && expected.line.is_none_or(|line| reported.line == Some(line))
        && expected
            .column
            .is_none_or(|column| reported.column == Some(column));
    if matches {
        None
    } else {
        Some(format!(
            "the location of the first load error is {}, expected {}",
            describe_location(
                reported.document,
                &reported.pointer,
                reported.line,
                reported.column
            ),
            describe_location(
                expected.document,
                &expected.pointer,
                expected.line,
                expected.column
            )
        ))
    }
}

/// A location as `document pointer line:column`, for failure reasons.
fn describe_location(
    document: InputDocument,
    pointer: &str,
    line: Option<u64>,
    column: Option<u64>,
) -> String {
    let position =
        |value: Option<u64>| value.map_or_else(|| "-".to_owned(), |value| value.to_string());
    format!(
        "{document:?} `{pointer}` {}:{}",
        position(line),
        position(column)
    )
}

/// A diagnostic as `CODE (message)`, for failure reasons.
pub(crate) fn describe(diagnostic: Option<&Diagnostic>) -> String {
    diagnostic.map_or_else(
        || "none".to_owned(),
        |diagnostic| format!("{} ({})", diagnostic.code, diagnostic.message),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs_2_8_each_expected_warning_needs_its_own_warning() {
        let warning = |code: &str| Diagnostic {
            code: code.to_owned(),
            message: String::new(),
            location: None,
        };
        let reported = [warning("W0F-01")];
        let expected = ["W0F-01".to_owned(), "W0F-01".to_owned()];
        assert!(missing_warnings(&expected, reported.iter()).is_some());
        assert!(missing_warnings(&expected[..1], reported.iter()).is_none());
    }

    fn expected_location(line: Option<u64>, column: Option<u64>) -> ExpectedLocation {
        ExpectedLocation {
            document: InputDocument::Model,
            pointer: "/a".to_owned(),
            line,
            column,
        }
    }

    fn diagnostic_location(line: Option<u64>, column: Option<u64>) -> DiagnosticLocation {
        DiagnosticLocation {
            document: InputDocument::Model,
            pointer: "/a".to_owned(),
            line,
            column,
        }
    }

    #[test]
    fn cs_2_9_a_first_error_without_location_fails() {
        assert!(location_mismatch(&expected_location(None, None), None).is_some());
    }

    #[test]
    fn cs_2_9_an_expected_line_needs_a_reported_line() {
        let reported = diagnostic_location(None, None);
        assert!(location_mismatch(&expected_location(Some(1), None), Some(&reported)).is_some());
        assert!(location_mismatch(&expected_location(None, Some(1)), Some(&reported)).is_some());
    }

    #[test]
    fn cs_2_9_line_and_column_are_compared_only_when_the_case_gives_them() {
        let reported = diagnostic_location(Some(3), Some(4));
        assert!(location_mismatch(&expected_location(None, None), Some(&reported)).is_none());
        assert!(location_mismatch(&expected_location(Some(3), Some(4)), Some(&reported)).is_none());
    }
}
