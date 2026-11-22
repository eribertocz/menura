//! The `pattern` and `format` assertions the contract schemas use.
//!
//! The schemas use a small, closed set of regular expressions. Each is matched by a dedicated
//! function instead of a regular expression engine; a pattern outside the set fails closed.

use crate::hash::is_lowercase_hex;

/// Matches `text` against one of the known schema patterns.
/// Returns `None` when the pattern is not one the contract schemas use.
pub(crate) fn matches_pattern(pattern: &str, text: &str) -> Option<bool> {
    let matched = match pattern {
        "^[0-9a-f]{64}$" => is_lowercase_hex(text, 64, 64),
        "^[0-9a-f]{7,40}$" => is_lowercase_hex(text, 7, 40),
        "^(E|W)[A-Z0-9_-]+$" => is_diagnostic_code(text),
        "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$" => is_semantic_version(text),
        "^([0-9]{4}|[0-9]{4}-Q[1-4]|[0-9]{4}-(0[1-9]|1[0-2])|[0-9]{4}-W(0[1-9]|[1-4][0-9]|5[0-3])|[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01]))$" => {
            is_period_shape(text)
        }
        "^(field|metric|dataset):.+$" => is_prefixed(text, &["field", "metric", "dataset"]),
        "^(dataset|field|metric|relationship):.+$" => {
            is_prefixed(text, &["dataset", "field", "metric", "relationship"])
        }
        "^(dataset|relationship|field|metric|policy):.+$" => is_prefixed(
            text,
            &["dataset", "relationship", "field", "metric", "policy"],
        ),
        "^dataset:.+$" => is_prefixed(text, &["dataset"]),
        "^relationship:.+$" => is_prefixed(text, &["relationship"]),
        "^field:.+$" => is_prefixed(text, &["field"]),
        "^metric:.+$" => is_prefixed(text, &["metric"]),
        "^policy:.+$" => is_prefixed(text, &["policy"]),
        "^menura\\.[a-z][a-z_]*$" => is_menura_contract_name(text),
        "^(/([^~/]|~[01])*)+$" => is_non_empty_json_pointer(text),
        "^1\\.[01]\\." => text.starts_with("1.0.") || text.starts_with("1.1."),
        _ => return None,
    };
    Some(matched)
}

/// Checks `text` against a JSON Schema `format`.
/// Returns `None` when the format is not one the contract schemas use.
pub(crate) fn matches_format(format: &str, text: &str) -> Option<bool> {
    match format {
        "date" => Some(is_full_date(text)),
        "date-time" => Some(is_date_time(text)),
        _ => None,
    }
}

/// One of `kinds`, then `:`, then at least one character, none of them a line terminator: the
/// regular expression `.` does not match one (ECMA-262).
fn is_prefixed(text: &str, kinds: &[&str]) -> bool {
    let rest = kinds.iter().find_map(|kind| {
        text.strip_prefix(kind)
            .and_then(|rest| rest.strip_prefix(':'))
    });
    rest.is_some_and(|rest| {
        !rest.is_empty()
            && !rest
                .chars()
                .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
    })
}

/// `menura.` followed by a lowercase ASCII letter, then lowercase ASCII letters and `_`.
fn is_menura_contract_name(text: &str) -> bool {
    let Some(local_name) = text.strip_prefix("menura.") else {
        return false;
    };
    let mut characters = local_name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| character.is_ascii_lowercase() || character == '_')
}

/// A non-empty JSON Pointer (RFC 6901): one or more `/`-prefixed reference tokens, where `~`
/// appears only as `~0` or `~1`.
fn is_non_empty_json_pointer(text: &str) -> bool {
    if !text.starts_with('/') {
        return false;
    }
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character == '~' && !matches!(characters.next(), Some('0' | '1')) {
            return false;
        }
    }
    true
}

fn is_diagnostic_code(text: &str) -> bool {
    let mut characters = text.chars();
    matches!(characters.next(), Some('E' | 'W'))
        && !characters.as_str().is_empty()
        && characters.all(|character| {
            character.is_ascii_uppercase()
                || character.is_ascii_digit()
                || character == '_'
                || character == '-'
        })
}

/// `MAJOR.MINOR.PATCH`, each a decimal number without leading zeros.
pub(crate) fn is_semantic_version(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    parts.len() == 3 && parts.iter().all(|part| is_unpadded_number(part))
}

fn is_unpadded_number(text: &str) -> bool {
    is_digits(text, 1, usize::MAX) && (text == "0" || !text.starts_with('0'))
}

fn is_digits(text: &str, minimum: usize, maximum: usize) -> bool {
    (minimum..=maximum).contains(&text.len()) && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// The shape of a canonical period (CAT-5.1): `YYYY`, `YYYY-Qn`, `YYYY-MM`, `YYYY-Www` or
/// `YYYY-MM-DD`, with the ranges of the schema pattern.
fn is_period_shape(text: &str) -> bool {
    let Some((year, rest)) = text.split_at_checked(4) else {
        return false;
    };
    if !is_digits(year, 4, 4) {
        return false;
    }
    if rest.is_empty() {
        return true;
    }
    let Some(rest) = rest.strip_prefix('-') else {
        return false;
    };
    if let Some(quarter) = rest.strip_prefix('Q') {
        return matches!(quarter, "1" | "2" | "3" | "4");
    }
    if let Some(week) = rest.strip_prefix('W') {
        return is_number_in(week, 1, 53);
    }
    match rest.split_once('-') {
        None => is_number_in(rest, 1, 12),
        Some((month, day)) => is_number_in(month, 1, 12) && is_number_in(day, 1, 31),
    }
}

/// Exactly two digits whose value is in `minimum..=maximum`.
fn is_number_in(text: &str, minimum: u32, maximum: u32) -> bool {
    is_digits(text, 2, 2)
        && text
            .parse::<u32>()
            .is_ok_and(|value| (minimum..=maximum).contains(&value))
}

/// RFC 3339 `full-date`: `YYYY-MM-DD`, a real calendar date.
pub(crate) fn is_full_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        return false;
    };
    if !(is_digits(year, 4, 4) && is_number_in(month, 1, 12) && is_digits(day, 2, 2)) {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (year.parse(), month.parse(), day.parse::<u32>()) else {
        return false;
    };
    (1..=days_in_month(year, month)).contains(&day)
}

/// Number of days of `month` (1–12) in the proleptic Gregorian `year`.
pub(crate) fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// RFC 3339 `date-time`: `full-date "T" partial-time time-offset`, where the offset is `Z` or
/// `±HH:MM`. `T` and `Z` may be lower case (RFC 3339 §5.6 note).
fn is_date_time(text: &str) -> bool {
    let Some((date, time)) = text.split_once(['T', 't']) else {
        return false;
    };
    if !is_full_date(date) {
        return false;
    }
    let (local, offset) = match time.find(['Z', 'z', '+', '-']) {
        Some(index) => time.split_at(index),
        None => return false,
    };
    is_partial_time(local) && is_time_offset(offset)
}

/// `HH:MM:SS[.fraction]`. Seconds may be 60 (leap second), as the RFC 3339 grammar allows.
fn is_partial_time(text: &str) -> bool {
    let (whole, fraction) = match text.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (text, None),
    };
    let parts: Vec<&str> = whole.split(':').collect();
    let [hour, minute, second] = parts.as_slice() else {
        return false;
    };
    is_two_digit_at_most(hour, 23)
        && is_two_digit_at_most(minute, 59)
        && is_two_digit_at_most(second, 60)
        && fraction.is_none_or(|fraction| is_digits(fraction, 1, usize::MAX))
}

fn is_time_offset(text: &str) -> bool {
    if text == "Z" || text == "z" {
        return true;
    }
    let Some(numeric) = text.strip_prefix(['+', '-']) else {
        return false;
    };
    match numeric.split_once(':') {
        Some((hour, minute)) => is_two_digit_at_most(hour, 23) && is_two_digit_at_most(minute, 59),
        None => false,
    }
}

fn is_two_digit_at_most(text: &str, maximum: u32) -> bool {
    is_digits(text, 2, 2) && text.parse::<u32>().is_ok_and(|value| value <= maximum)
}

#[cfg(test)]
mod tests {
    use super::{matches_format, matches_pattern};

    #[test]
    fn period_pattern_shapes() {
        let pattern = "^([0-9]{4}|[0-9]{4}-Q[1-4]|[0-9]{4}-(0[1-9]|1[0-2])|[0-9]{4}-W(0[1-9]|[1-4][0-9]|5[0-3])|[0-9]{4}-(0[1-9]|1[0-2])-(0[1-9]|[12][0-9]|3[01]))$";
        for valid in ["2026", "2026-Q4", "2026-10", "2020-W53", "2026-10-31"] {
            assert_eq!(matches_pattern(pattern, valid), Some(true), "{valid}");
        }
        for invalid in [
            "26",
            "2026-Q5",
            "2026-13",
            "2026-W54",
            "2026-W00",
            "2026-10-32",
            "2026-1",
        ] {
            assert_eq!(matches_pattern(pattern, invalid), Some(false), "{invalid}");
        }
    }

    #[test]
    fn semantic_version_and_code_patterns() {
        let semver = "^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$";
        assert_eq!(matches_pattern(semver, "1.0.0"), Some(true));
        assert_eq!(matches_pattern(semver, "01.0.0"), Some(false));
        assert_eq!(matches_pattern(semver, "1.0"), Some(false));
        let code = "^(E|W)[A-Z0-9_-]+$";
        assert_eq!(matches_pattern(code, "E_NAME_NOT_FOUND"), Some(true));
        assert_eq!(matches_pattern(code, "E"), Some(false));
        assert_eq!(matches_pattern(code, "Xabc"), Some(false));
        assert_eq!(matches_pattern("^unknown$", "x"), None);
    }

    #[test]
    fn alternative_contract_name_pointer_and_version_prefix_patterns() {
        let alternative = "^(field|metric|dataset):.+$";
        assert_eq!(matches_pattern(alternative, "metric:billed"), Some(true));
        assert_eq!(matches_pattern(alternative, "dataset:a b"), Some(true));
        assert_eq!(matches_pattern(alternative, "metric:"), Some(false));
        assert_eq!(matches_pattern(alternative, "metric:a\nb"), Some(false));
        assert_eq!(matches_pattern(alternative, "column:a"), Some(false));
        let contract = "^menura\\.[a-z][a-z_]*$";
        assert_eq!(
            matches_pattern(contract, "menura.compiled_plan"),
            Some(true)
        );
        assert_eq!(matches_pattern(contract, "menura._plan"), Some(false));
        assert_eq!(matches_pattern(contract, "menura.Plan"), Some(false));
        assert_eq!(
            matches_pattern(contract, "semantic_compiler.query"),
            Some(false)
        );
        let pointer = "^(/([^~/]|~[01])*)+$";
        assert_eq!(
            matches_pattern(pointer, "/datasets/0/ai_context"),
            Some(true)
        );
        assert_eq!(matches_pattern(pointer, "/a~0b/~1/"), Some(true));
        assert_eq!(matches_pattern(pointer, ""), Some(false));
        assert_eq!(matches_pattern(pointer, "a/b"), Some(false));
        assert_eq!(matches_pattern(pointer, "/a~2"), Some(false));
        assert_eq!(matches_pattern(pointer, "/a~"), Some(false));
        let prefix = "^1\\.[01]\\.";
        assert_eq!(matches_pattern(prefix, "1.1.0"), Some(true));
        assert_eq!(matches_pattern(prefix, "1.0.7"), Some(true));
        assert_eq!(matches_pattern(prefix, "1.2.0"), Some(false));
        assert_eq!(matches_pattern(prefix, "1.10.0"), Some(false));
    }

    #[test]
    fn date_and_date_time_formats() {
        assert_eq!(matches_format("date", "2024-02-29"), Some(true));
        assert_eq!(matches_format("date", "2026-02-29"), Some(false));
        assert_eq!(
            matches_format("date-time", "2026-11-15T12:00:00-04:00"),
            Some(true)
        );
        assert_eq!(
            matches_format("date-time", "2026-11-15T12:00:00.5Z"),
            Some(true)
        );
        assert_eq!(
            matches_format("date-time", "2026-11-15T12:00:00"),
            Some(false)
        );
        assert_eq!(
            matches_format("date-time", "2026-11-15 12:00:00Z"),
            Some(false)
        );
        assert_eq!(matches_format("uri", "x"), None);
    }
}
