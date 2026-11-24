//! The literals the `ClickHouse` generator writes on purpose (E6-4.11, E6-7): constants of the
//! dialect whose value can equal a query value by chance, so the check of E6-4.2 does not take
//! them for leaked values. Each is the `ClickHouse` form of a constant `DuckDB` writes too, or a
//! constant that chapter E6-7 introduces.
//!
//! - every value of the final `SETTINGS` clause (E6-7.8);
//! - the `'UTC'` time zone of a `DateTime64` type and of `toTimeZone`, which E6-7.2 fixes for
//!   every timestamp with time zone, and the precision of `DateTime64` and `toDateTime64`;
//! - the integer of `addDays`, `addMonths` and the like in the period arithmetic, as the
//!   `INTERVAL n <unit>` of `DuckDB` (E6-4.12);
//! - the `0` of `coalesce(count(…), 0)`, which keeps `count` at 0 over no rows although
//!   `aggregate_functions_null_for_empty` makes aggregates NULL there (E6-7.8).

use crate::sql_literals::{Literal, LiteralKind};

/// The line that opens the final `SETTINGS` clause of a statement (E6-7.8).
const SETTINGS_LINE: &str = "\nSETTINGS\n";

/// The time zone of every `ClickHouse` timestamp type (E6-7.2).
const TYPE_TIME_ZONE: &str = "UTC";

/// The function arguments that hold the time zone of a type: `DateTime64(6, 'UTC')` and
/// `toTimeZone(x, 'UTC')`, as a function and an argument index from 0.
const TIME_ZONE_ARGUMENTS: [(&str, usize); 2] = [("DATETIME64", 1), ("TOTIMEZONE", 1)];

/// The function arguments that hold the precision of a timestamp type.
const PRECISION_ARGUMENTS: [(&str, usize); 2] = [("DATETIME64", 0), ("TODATETIME64", 1)];

/// The functions of the period arithmetic, whose argument 1 is the number of units.
const PERIOD_FUNCTIONS: [&str; 10] = [
    "ADDDAYS",
    "ADDWEEKS",
    "ADDMONTHS",
    "ADDQUARTERS",
    "ADDYEARS",
    "SUBTRACTDAYS",
    "SUBTRACTWEEKS",
    "SUBTRACTMONTHS",
    "SUBTRACTQUARTERS",
    "SUBTRACTYEARS",
];

/// Whether `literal` of the `ClickHouse` statement `sql` is one of the constants of this
/// module.
pub(crate) fn is_clickhouse_constant(literal: &Literal, sql: &str) -> bool {
    in_settings_clause(literal, sql)
        || is_type_time_zone(literal)
        || is_number_argument(literal, &PRECISION_ARGUMENTS)
        || is_period_count(literal)
        || is_empty_count(literal, sql)
}

/// Whether `literal` is in the `SETTINGS` clause that ends the statement.
fn in_settings_clause(literal: &Literal, sql: &str) -> bool {
    sql[..literal.offset]
        .rfind(SETTINGS_LINE)
        .is_some_and(|start| !sql[start..].contains("SELECT"))
}

fn is_type_time_zone(literal: &Literal) -> bool {
    literal.kind == LiteralKind::Text
        && literal.value == TYPE_TIME_ZONE
        && literal.call.as_ref().is_some_and(|(function, argument)| {
            TIME_ZONE_ARGUMENTS.contains(&(function.as_str(), *argument))
        })
}

fn is_number_argument(literal: &Literal, arguments: &[(&str, usize)]) -> bool {
    literal.kind == LiteralKind::Number
        && literal
            .call
            .as_ref()
            .is_some_and(|(function, argument)| arguments.contains(&(function.as_str(), *argument)))
}

fn is_period_count(literal: &Literal) -> bool {
    literal.kind == LiteralKind::Number
        && literal.call.as_ref().is_some_and(|(function, argument)| {
            *argument == 1 && PERIOD_FUNCTIONS.contains(&function.as_str())
        })
}

/// Whether `literal` is the `0` of `coalesce(count(…), 0)`.
fn is_empty_count(literal: &Literal, sql: &str) -> bool {
    let is_coalesce_default = literal.kind == LiteralKind::Number
        && literal.value == "0"
        && literal
            .call
            .as_ref()
            .is_some_and(|(function, argument)| function == "COALESCE" && *argument == 1);
    if !is_coalesce_default {
        return false;
    }
    let before = sql[..literal.offset].trim_end();
    let Some(first) = before.strip_suffix(',') else {
        return false;
    };
    let Some(open) = first.rfind("coalesce(") else {
        return false;
    };
    first[open + "coalesce(".len()..].starts_with("count(")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql_literals::literals;

    /// Whether every literal of `sql` that equals `value` is a constant of this module.
    fn constants(sql: &str, value: &str) -> Vec<bool> {
        literals(sql)
            .iter()
            .filter(|literal| literal.value == value)
            .map(|literal| is_clickhouse_constant(literal, sql))
            .collect()
    }

    #[test]
    fn e6_4_2_settings_values_are_constants() {
        let sql = "SELECT \"a\" FROM \"t\" WHERE \"a\" = 1\nSETTINGS\n  join_use_nulls = 1";
        assert_eq!(constants(sql, "1"), vec![false, true]);
    }

    #[test]
    fn e6_4_2_the_utc_of_a_type_is_a_constant_but_a_utc_value_is_not() {
        let sql = "SELECT toTimeZone(x, 'UTC'), CAST(y AS DateTime64(6, 'UTC')), 'UTC' FROM t";
        assert_eq!(constants(sql, "UTC"), vec![true, true, false]);
    }

    #[test]
    fn e6_4_2_period_counts_and_empty_counts_are_constants() {
        let sql =
            "SELECT addDays(addMonths(p, 1), -1), coalesce(count(x), 0), coalesce(y, 0) FROM t";
        assert_eq!(constants(sql, "1"), vec![true]);
        assert_eq!(constants(sql, "0"), vec![true, false]);
    }
}
