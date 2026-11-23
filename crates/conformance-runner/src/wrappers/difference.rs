//! How a difference between a wrapper and `core` is reported: the case, the wrapper and the
//! first differing line (ARC-6.9).

use std::fmt;

/// The number of characters shown on each side of the first differing column.
const EXCERPT_RADIUS: usize = 40;

/// The first line where two texts differ, from 1, with an excerpt of each side around the first
/// differing column. A side that has no such line is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineDifference {
    /// The line number, from 1.
    pub line: usize,
    /// The column of the first differing character, from 1.
    pub column: usize,
    /// The excerpt of the expected line.
    pub expected: Option<String>,
    /// The excerpt of the actual line.
    pub actual: Option<String>,
}

impl fmt::Display for LineDifference {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side = |text: &Option<String>| match text {
            Some(text) => format!("`{text}`"),
            None => "no line".to_owned(),
        };
        write!(
            formatter,
            "line {}, column {}: expected {}, found {}",
            self.line,
            self.column,
            side(&self.expected),
            side(&self.actual)
        )
    }
}

/// The first line where `actual` differs from `expected`, or `None` when they are equal.
///
/// # Example
///
/// ```
/// use menura_conformance_runner::wrappers::first_difference;
///
/// let difference = first_difference("{\n  \"a\": 1\n}", "{\n  \"a\": 2\n}").unwrap();
/// assert_eq!((difference.line, difference.column), (2, 9));
/// assert!(first_difference("same", "same").is_none());
/// ```
#[must_use]
pub fn first_difference(expected: &str, actual: &str) -> Option<LineDifference> {
    if expected == actual {
        return None;
    }
    let mut expected_lines = expected.split('\n');
    let mut actual_lines = actual.split('\n');
    let mut line = 1;
    loop {
        match (expected_lines.next(), actual_lines.next()) {
            (Some(left), Some(right)) if left == right => line += 1,
            (left, right) => return Some(line_difference(line, left, right)),
        }
    }
}

/// The difference of one line, with excerpts around its first differing column.
fn line_difference(line: usize, expected: Option<&str>, actual: Option<&str>) -> LineDifference {
    let left: Vec<char> = expected.unwrap_or_default().chars().collect();
    let right: Vec<char> = actual.unwrap_or_default().chars().collect();
    let column = left
        .iter()
        .zip(&right)
        .take_while(|(left, right)| left == right)
        .count();
    let excerpt = |characters: &[char]| {
        let start = column.saturating_sub(EXCERPT_RADIUS);
        let end = (column + EXCERPT_RADIUS).min(characters.len());
        characters
            .get(start..end)
            .unwrap_or_default()
            .iter()
            .collect()
    };
    LineDifference {
        line,
        column: column + 1,
        expected: expected.map(|_| excerpt(&left)),
        actual: actual.map(|_| excerpt(&right)),
    }
}

/// A difference between what a wrapper gave for a call and what `core` gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mismatch {
    /// The case of the call.
    pub case: String,
    /// The wrapper: `cli` or `mcp`.
    pub wrapper: &'static str,
    /// The command of the call.
    pub command: &'static str,
    /// What differs, with the first differing line of a document.
    pub detail: String,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "case {}: {} `{}`: {}",
            self.case, self.wrapper, self.command, self.detail
        )
    }
}

/// The mismatch of `part` when `actual` differs from `expected`, if it does.
pub(crate) fn compare_text(part: &str, expected: &str, actual: &str) -> Option<String> {
    first_difference(expected, actual).map(|difference| format!("{part}: {difference}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_6_9_an_injected_difference_reports_its_first_differing_line() {
        let expected = "{\n  \"sql\": \"SELECT 1\",\n  \"x\": 1\n}";
        let actual = expected.replace("SELECT 1", "SELECT 2");
        let difference = first_difference(expected, &actual).unwrap();
        assert_eq!(difference.line, 2);
        assert_eq!(difference.column, 18);
        assert_eq!(
            difference.to_string(),
            "line 2, column 18: expected `  \"sql\": \"SELECT 1\",`, found `  \"sql\": \"SELECT 2\",`"
        );
    }

    #[test]
    fn arc_6_9_a_missing_final_line_is_a_difference() {
        let difference = first_difference("a\n", "a").unwrap();
        assert_eq!(difference.line, 2);
        assert_eq!(difference.actual, None);
        assert_eq!(difference.expected.as_deref(), Some(""));
    }

    #[test]
    fn arc_6_9_equal_texts_have_no_difference() {
        assert_eq!(compare_text("document", "a\nb", "a\nb"), None);
    }
}
