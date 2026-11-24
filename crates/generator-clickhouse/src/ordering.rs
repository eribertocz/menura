//! Comparisons and ordering with `DuckDB`'s semantics where `ClickHouse`'s differ (E6-7.14): a
//! float NaN equals NaN and is greater than every other value, and a `uuid` compares and sorts
//! as its canonical text.

use menura_contracts::types::common::LogicalType;

/// How the values of a type compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Comparison {
    /// As `ClickHouse` compares them.
    Plain,
    /// Floats: NaN equals NaN and is the greatest value.
    Float,
    /// UUIDs: as their canonical text.
    Text,
}

impl Comparison {
    /// How two values of `left` and `right` compare: as floats when either is a float, as text
    /// when either is a `uuid`.
    pub(crate) fn of(left: Option<&LogicalType>, right: Option<&LogicalType>) -> Self {
        let any =
            |test: fn(&LogicalType) -> bool| left.is_some_and(test) || right.is_some_and(test);
        if any(|logical_type| matches!(logical_type, LogicalType::Float { .. })) {
            Self::Float
        } else if any(|logical_type| matches!(logical_type, LogicalType::Uuid)) {
            Self::Text
        } else {
            Self::Plain
        }
    }
}

/// `left <operator> right`, compared as `comparison` says. A float compares as the pair
/// `(isNaN(x), x without NaN)`, which orders NaN last and makes two NaN equal; the pairs are
/// compared only when neither side is NULL, so a NULL still gives NULL.
pub(crate) fn compared(comparison: Comparison, left: &str, operator: &str, right: &str) -> String {
    match comparison {
        Comparison::Plain => format!("{left} {operator} {right}"),
        Comparison::Text => format!("toString({left}) {operator} toString({right})"),
        Comparison::Float => format!(
            "CASE WHEN {left} IS NULL OR {right} IS NULL THEN NULL \
             ELSE {} {operator} {} END",
            float_key(left),
            float_key(right)
        ),
    }
}

/// The ordering terms of `column`, of `logical_type`, in `direction` with NULL values where
/// `nulls` says (E6-7.15): a float first by whether it is NULL, then by whether it is NaN, so
/// that NaN sorts after every other value; a `uuid` by its text.
pub(crate) fn sort_terms(
    column: &str,
    logical_type: &LogicalType,
    direction: &str,
    nulls: &str,
) -> String {
    match logical_type {
        LogicalType::Float { .. } => {
            let null_direction = if nulls == "NULLS FIRST" {
                "DESC"
            } else {
                "ASC"
            };
            format!(
                "{column} IS NULL {null_direction} NULLS LAST, \
                 isNaN({column}) {direction} NULLS LAST, {column} {direction} {nulls}"
            )
        }
        LogicalType::Uuid => format!("toString({column}) {direction} {nulls}"),
        _ => format!("{column} {direction} {nulls}"),
    }
}

/// The pair a float compares as: `(isNaN(x), if(isNaN(x), 0, x))`.
fn float_key(value: &str) -> String {
    format!("(isNaN({value}), if(isNaN({value}), 0, {value}))")
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::{FloatBits, LogicalType};

    use super::{Comparison, compared, sort_terms};

    #[test]
    fn e6_7_14_nan_equals_nan_and_sorts_last() {
        let float = LogicalType::Float {
            bits: FloatBits::SixtyFour,
        };
        assert_eq!(
            compared(Comparison::of(Some(&float), None), "\"a\"", "=", "\"b\""),
            "CASE WHEN \"a\" IS NULL OR \"b\" IS NULL THEN NULL \
             ELSE (isNaN(\"a\"), if(isNaN(\"a\"), 0, \"a\")) = (isNaN(\"b\"), if(isNaN(\"b\"), 0, \"b\")) END"
        );
        assert_eq!(
            sort_terms("\"a\"", &float, "ASC", "NULLS FIRST"),
            "\"a\" IS NULL DESC NULLS LAST, isNaN(\"a\") ASC NULLS LAST, \"a\" ASC NULLS FIRST"
        );
    }

    #[test]
    fn e6_7_14_uuids_compare_and_sort_as_text() {
        assert_eq!(
            compared(
                Comparison::of(None, Some(&LogicalType::Uuid)),
                "\"a\"",
                "<",
                "\"b\""
            ),
            "toString(\"a\") < toString(\"b\")"
        );
        assert_eq!(
            sort_terms("\"a\"", &LogicalType::Uuid, "DESC", "NULLS LAST"),
            "toString(\"a\") DESC NULLS LAST"
        );
    }
}
