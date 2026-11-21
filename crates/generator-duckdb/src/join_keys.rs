//! The condition of a `Join`: its key pairs compared for equality, each pair of different
//! types in their common type, with explicit casts on both sides (E6-4.13, E6-4.10).

use menura_contracts::types::common::LogicalType;
use menura_expressions::typing::{Aggregates, Outcome, Stop, Typed, TypingScope, type_expression};
use menura_model::expression::Reference;

use crate::types::physical_type;

/// One side of a key pair: its SQL and its logical type.
#[derive(Debug, Clone, Copy)]
pub(crate) struct KeySide<'key> {
    /// The SQL of the key column, such as `"menura_1_scan"."CUSTOMER_ID"`.
    pub(crate) sql: &'key str,
    /// Its logical type.
    pub(crate) logical_type: &'key LogicalType,
}

/// `left = right`, or `CAST(left AS T) = CAST(right AS T)` when the types differ, `T` the
/// common type of E0F-3.1. `None` when the two types have no common type or `T` has no
/// `DuckDB` name.
pub(crate) fn key_equality(left: KeySide<'_>, right: KeySide<'_>) -> Option<String> {
    if left.logical_type == right.logical_type {
        return Some(format!("{} = {}", left.sql, right.sql));
    }
    let common = comparison_type(left.logical_type, right.logical_type)?;
    let physical = physical_type(&common)?;
    Some(format!(
        "CAST({} AS {physical}) = CAST({} AS {physical})",
        left.sql, right.sql
    ))
}

/// The type in which a value of type `left` and one of type `right` are compared (E0F-3.1):
/// for two numeric types, the type E0F-5a gives their `-`; otherwise the CASE rule of E0F-5a.
/// Both are read from the typer of E0.f, so that E6 uses exactly its conversions.
pub(crate) fn comparison_type(left: &LogicalType, right: &LogicalType) -> Option<LogicalType> {
    let text = if is_numeric(left) && is_numeric(right) {
        "LEFT_KEY - RIGHT_KEY"
    } else {
        "CASE WHEN TRUE THEN LEFT_KEY ELSE RIGHT_KEY END"
    };
    let expression = menura_expressions::parse(text).ok()?;
    let resolve = |reference: &Reference| -> Outcome<Typed> {
        match reference.name.text() {
            "LEFT_KEY" => Ok(Typed::value(left.clone(), true)),
            "RIGHT_KEY" => Ok(Typed::value(right.clone(), true)),
            _ => Err(Stop::ReportedElsewhere),
        }
    };
    let scope = TypingScope::new(&resolve, Aggregates::NotAllowed);
    let typed = type_expression(&expression, &scope).ok()?;
    typed.logical().cloned()
}

fn is_numeric(logical_type: &LogicalType) -> bool {
    matches!(
        logical_type,
        LogicalType::Integer { .. } | LogicalType::Decimal { .. } | LogicalType::Float { .. }
    )
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::{IntegerBits, LogicalType};

    use super::{KeySide, key_equality};

    fn integer(bits: IntegerBits) -> LogicalType {
        LogicalType::Integer { bits, signed: true }
    }

    fn side<'key>(sql: &'key str, logical_type: &'key LogicalType) -> KeySide<'key> {
        KeySide { sql, logical_type }
    }

    #[test]
    fn e6_4_13_keys_of_one_type_are_compared_without_casts() {
        let big = integer(IntegerBits::SixtyFour);
        assert_eq!(
            key_equality(side("\"L\"", &big), side("\"R\"", &big)).as_deref(),
            Some("\"L\" = \"R\"")
        );
    }

    fn decimal(precision: u8, scale: u8) -> LogicalType {
        LogicalType::Decimal { precision, scale }
    }

    /// The condition of the key pair `left`, `right`, from both sides.
    fn both_ways(left: &LogicalType, right: &LogicalType) -> (Option<String>, Option<String>) {
        (
            key_equality(side("\"L\"", left), side("\"R\"", right)),
            key_equality(side("\"R\"", right), side("\"L\"", left)),
        )
    }

    #[test]
    fn e6_4_13_integer_keys_of_two_widths_are_cast_to_their_common_type() {
        let (forward, backward) = both_ways(
            &integer(IntegerBits::ThirtyTwo),
            &integer(IntegerBits::SixtyFour),
        );
        assert_eq!(
            forward.as_deref(),
            Some("CAST(\"L\" AS HUGEINT) = CAST(\"R\" AS HUGEINT)")
        );
        assert_eq!(
            backward.as_deref(),
            Some("CAST(\"R\" AS HUGEINT) = CAST(\"L\" AS HUGEINT)")
        );
    }

    #[test]
    fn e6_4_13_decimal_keys_of_two_sizes_are_cast_to_their_common_type() {
        let (forward, backward) = both_ways(&decimal(10, 2), &decimal(12, 2));
        assert_eq!(
            forward.as_deref(),
            Some("CAST(\"L\" AS DECIMAL(13,2)) = CAST(\"R\" AS DECIMAL(13,2))")
        );
        assert_eq!(
            backward.as_deref(),
            Some("CAST(\"R\" AS DECIMAL(13,2)) = CAST(\"L\" AS DECIMAL(13,2))")
        );
    }

    #[test]
    fn e6_4_13_keys_without_a_common_type_have_no_condition() {
        let big = integer(IntegerBits::SixtyFour);
        assert!(key_equality(side("\"L\"", &big), side("\"R\"", &LogicalType::Text)).is_none());
    }
}
