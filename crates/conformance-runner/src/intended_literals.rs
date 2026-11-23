//! The literals of a SQL text that are written on purpose rather than bound (E6-4.11): they are
//! never values of the query or of the security context that leaked into a plan.

use crate::sql_literals::{Literal, LiteralKind};

/// The keyword whose integer is a literal on purpose: `LIMIT` integers are values from a closed
/// set, generated as literals (E6-4.11).
const LIMIT_KEYWORD: &str = "LIMIT";

/// The keyword of the interval constants of the generator (see [`is_generator_constant`]).
const INTERVAL_KEYWORD: &str = "INTERVAL";

/// The keyword of a window, whose first row the generator keeps with `= 1`.
const WINDOW_KEYWORD: &str = " OVER";

/// The type whose arguments are part of a type name (see [`is_generator_constant`]).
const DECIMAL_TYPE: &str = "DECIMAL";

/// The function arguments that are literals on purpose (E6-4.11): the grain name of
/// `DATE_TRUNC`, the precision of `ROUND` and `TRUNC`, the part number of `SPLIT_PART` and the
/// fraction of `PERCENTILE_CONT` and `PERCENTILE_DISC`, each as a function and an argument
/// index from 0.
const CLOSED_ARGUMENTS: [(&str, usize); 6] = [
    ("DATE_TRUNC", 0),
    ("ROUND", 1),
    ("TRUNC", 1),
    ("SPLIT_PART", 2),
    ("PERCENTILE_CONT", 0),
    ("PERCENTILE_DISC", 0),
];

/// Whether a literal of an expression is written as a literal on purpose rather than bound:
/// a `LIMIT` integer, an integer argument that fixes a result type or a part, a grain name
/// (E6-4.11), or the position of a `BY` list item, which names a column rather than a value.
pub(crate) fn is_generated_on_purpose(literal: &Literal) -> bool {
    literal.ordinal
        || literal.preceding_word.as_deref() == Some(LIMIT_KEYWORD)
        || literal.call.as_ref().is_some_and(|(function, argument)| {
            CLOSED_ARGUMENTS.contains(&(function.as_str(), *argument))
        })
}

/// Whether a literal of the generated SQL is a constant that E6 introduces (E6-4.11), in one of
/// the shapes whose value can equal a query value:
///
/// - the precision and scale of a `DECIMAL(p,s)` type of a typed cast, which are part of the
///   `DuckDB` type name, not values (E6-4.2c, E6-5.6);
/// - the `0` of the division guard `CASE WHEN <divisor> = 0 THEN NULL ELSE … END`, by which a
///   division by zero gives `NULL` (E6-4.10, E0F-5a);
/// - the integer of an `INTERVAL n <unit>` of the period arithmetic, and the `1` of
///   `… OVER (…) = 1` that keeps the first row of a window, in the nodes that have model origin,
///   whose constants are literals (E6-4.12, E6-4.14).
pub(crate) fn is_generator_constant(literal: &Literal, sql: &str) -> bool {
    if literal.kind != LiteralKind::Number {
        return false;
    }
    let before = sql[..literal.offset].trim_end();
    let type_argument = literal
        .call
        .as_ref()
        .is_some_and(|(function, _)| function == DECIMAL_TYPE);
    let division_guard = literal.value == "0"
        && before.ends_with('=')
        && sql[literal.end..].trim_start().starts_with("THEN NULL");
    let first_of_window = literal.value == "1"
        && before
            .strip_suffix('=')
            .is_some_and(|text| closes_window(text.trim_end()));
    type_argument
        || division_guard
        || first_of_window
        || literal.preceding_word.as_deref() == Some(INTERVAL_KEYWORD)
}

/// Whether `text` ends with the parenthesis that closes an `OVER (…)` window.
fn closes_window(text: &str) -> bool {
    let Some(inside) = text.strip_suffix(')') else {
        return false;
    };
    let mut depth = 0_usize;
    for (offset, character) in inside.char_indices().rev() {
        match character {
            ')' => depth += 1,
            '(' if depth == 0 => {
                return inside[..offset]
                    .trim_end()
                    .to_ascii_uppercase()
                    .ends_with(WINDOW_KEYWORD);
            }
            '(' => depth -= 1,
            _ => {}
        }
    }
    false
}
