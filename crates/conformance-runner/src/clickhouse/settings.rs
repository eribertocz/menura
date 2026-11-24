//! The settings of E6-7.8 and their reversal in the session (E6-7.10).
//!
//! The runner sets each setting the generated SQL fixes to the opposite value at the session
//! level, so that a statement that does not fix one of them returns other rows. So that this
//! list cannot drift from the generator's, every compiled plan's `SETTINGS` clause must name
//! exactly these settings, each with the value the statement needs; the runner depends on no
//! stage's internals, so it reads the clause from the plan's `sql`.

/// A setting of the `SETTINGS` clause: the value the statement fixes, and the opposite value
/// the session sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReversedSetting {
    /// The setting name.
    pub(crate) name: &'static str,
    /// The value the generated statement fixes (E6-7.8).
    pub(crate) statement: &'static str,
    /// The value the session sets (E6-7.10).
    pub(crate) session: &'static str,
}

const fn reversed(
    name: &'static str,
    statement: &'static str,
    session: &'static str,
) -> ReversedSetting {
    ReversedSetting {
        name,
        statement,
        session,
    }
}

/// Every setting of E6-7.8, in the order of the generated clause.
pub(crate) const REVERSED_SETTINGS: [ReversedSetting; 9] = [
    reversed("join_use_nulls", "1", "0"),
    reversed("transform_null_in", "0", "1"),
    reversed("prefer_column_name_to_alias", "1", "0"),
    reversed("enable_positional_arguments", "1", "0"),
    reversed(
        "short_circuit_function_evaluation",
        "'force_enable'",
        "'disable'",
    ),
    reversed("enable_extended_results_for_datetime_functions", "1", "0"),
    reversed("aggregate_functions_null_for_empty", "1", "0"),
    reversed("decimal_check_overflow", "1", "0"),
    reversed("use_variant_as_common_type", "0", "1"),
];

/// The line that opens the final `SETTINGS` clause of a statement (E6-7.8).
const SETTINGS_LINE: &str = "\nSETTINGS\n";

/// Why the `SETTINGS` clause of `sql` does not name exactly the settings the session reverses,
/// each with the value the statement needs, or `None` (E6-7.8, E6-7.10).
pub(crate) fn settings_mismatch(sql: &str) -> Option<String> {
    let Some(start) = sql.rfind(SETTINGS_LINE) else {
        return Some("the statement has no final `SETTINGS` clause (E6-7.8)".to_owned());
    };
    let clause = &sql[start + SETTINGS_LINE.len()..];
    let mut written: Vec<(String, String)> = Vec::new();
    for item in clause.split(',') {
        let Some((name, value)) = item.split_once('=') else {
            return Some(format!(
                "`{}` is not a setting `name = value` (E6-7.8)",
                item.trim()
            ));
        };
        written.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    let expected: Vec<(String, String)> = REVERSED_SETTINGS
        .iter()
        .map(|setting| (setting.name.to_owned(), setting.statement.to_owned()))
        .collect();
    let mut sorted_written = written.clone();
    sorted_written.sort();
    let mut sorted_expected = expected;
    sorted_expected.sort();
    (sorted_written != sorted_expected).then(|| {
        let describe = |settings: &[(String, String)]| {
            settings
                .iter()
                .map(|(name, value)| format!("{name} = {value}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "the `SETTINGS` clause is [{}], but the session reverses [{}] (E6-7.10)",
            describe(&written),
            describe(&sorted_expected)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clause(settings: &[ReversedSetting]) -> String {
        let items: Vec<String> = settings
            .iter()
            .map(|setting| format!("  {} = {}", setting.name, setting.statement))
            .collect();
        format!("SELECT 1\nSETTINGS\n{}", items.join(",\n"))
    }

    #[test]
    fn e6_7_10_the_session_value_of_every_setting_is_the_opposite_of_the_statement_value() {
        for setting in REVERSED_SETTINGS {
            assert_ne!(setting.statement, setting.session, "{}", setting.name);
        }
    }

    #[test]
    fn e6_7_10_accepts_a_clause_with_exactly_the_reversed_settings() {
        assert_eq!(settings_mismatch(&clause(&REVERSED_SETTINGS)), None);
    }

    #[test]
    fn e6_7_10_fails_a_clause_with_a_setting_the_session_does_not_reverse() {
        let mut sql = clause(&REVERSED_SETTINGS);
        sql.push_str(",\n  max_threads = 1");
        assert!(settings_mismatch(&sql).is_some_and(|reason| reason.contains("max_threads")));
    }

    #[test]
    fn e6_7_10_fails_a_clause_without_a_reversed_setting_or_with_another_value() {
        assert!(settings_mismatch(&clause(&REVERSED_SETTINGS[1..])).is_some());
        let mut changed = REVERSED_SETTINGS;
        changed[0] = reversed("join_use_nulls", "0", "1");
        assert!(settings_mismatch(&clause(&changed)).is_some());
    }

    #[test]
    fn e6_7_10_fails_a_statement_without_a_settings_clause() {
        assert!(settings_mismatch("SELECT 1").is_some());
    }
}
