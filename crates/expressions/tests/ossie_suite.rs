//! E0F-9.1: the official Apache Ossie expression suite, vendored at the tracked commit in
//! `conformance/ossie/compliance/expressions/`. No line of `valid.txt` gives `E0F-01`, and every
//! line of `invalid.txt` does.

use menura_expressions::{ExpressionCode, parse};

const VALID: &str = include_str!("../../../conformance/ossie/compliance/expressions/valid.txt");
const INVALID: &str = include_str!("../../../conformance/ossie/compliance/expressions/invalid.txt");

/// The expressions of a suite file: blank lines and lines starting with `#` are skipped, as the
/// file's header says.
fn expressions(suite: &str) -> Vec<&str> {
    suite
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .collect()
}

/// Whether `text` gives `E0F-01`.
fn is_unparsable(text: &str) -> bool {
    parse(text).is_err_and(|error| error.code() == ExpressionCode::Unparsable)
}

#[test]
fn e0f_9_1_no_valid_line_gives_e0f_01() {
    let lines = expressions(VALID);
    assert!(
        lines.len() > 50,
        "valid.txt has {} expressions",
        lines.len()
    );
    let failures: Vec<String> = lines
        .iter()
        .filter(|line| is_unparsable(line))
        .map(|line| format!("{line}: {:?}", parse(line).err()))
        .collect();
    assert!(
        failures.is_empty(),
        "lines of valid.txt that give E0F-01:\n{}",
        failures.join("\n")
    );
}

#[test]
fn e0f_9_1_every_invalid_line_gives_e0f_01() {
    let lines = expressions(INVALID);
    assert!(
        lines.len() > 10,
        "invalid.txt has {} expressions",
        lines.len()
    );
    let failures: Vec<String> = lines
        .iter()
        .filter(|line| !is_unparsable(line))
        .map(|line| format!("{line}: {:?}", parse(line)))
        .collect();
    assert!(
        failures.is_empty(),
        "lines of invalid.txt that do not give E0F-01:\n{}",
        failures.join("\n")
    );
}
