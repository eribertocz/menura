//! Runtime errors raised in SQL with `throwIf` (E6-7.5, E6-4.6a, E6-4.6b).

use crate::values::text_sql;

/// `throwIf(ifNull(condition, 0), 'message')`: raises `message`, a literal (E6-4.11), for a
/// row where `condition` holds, and is 0 otherwise. A NULL condition does not raise.
/// `ClickHouse` requires the message to be a constant, and evaluates a `CASE` branch only for
/// the rows that reach it under the `short_circuit_function_evaluation` setting of E6-7.8.
pub(crate) fn raise(condition: &str, message: &str) -> String {
    format!("throwIf(ifNull({condition}, 0), {})", text_sql(message))
}

#[cfg(test)]
mod tests {
    use super::raise;

    #[test]
    fn e6_7_5_a_condition_is_raised_with_throw_if() {
        assert_eq!(
            raise("\"d\" = 0", "RDER-01: division by zero in metric M"),
            "throwIf(ifNull(\"d\" = 0, 0), 'RDER-01: division by zero in metric M')"
        );
    }
}
