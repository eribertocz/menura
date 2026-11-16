//! Reads one case file and checks the rules of CS-1 and CS-2 that a case must meet by itself.

use std::fs;
use std::path::Path;

use crate::case::{Case, Outcome};
use crate::error::{RunnerError, RunnerErrorKind};

/// Reads and validates the case file at `path`.
///
/// # Errors
///
/// Returns a [`RunnerError`] when the file cannot be read or parsed, when its name differs from
/// its `id` (CS-1), or when it breaks CS-2.1, CS-2.2 or the security context requirement of CS-2.
pub fn read_case(path: &Path) -> Result<Case, RunnerError> {
    let text = fs::read_to_string(path)
        .map_err(|error| RunnerError::new(RunnerErrorKind::Unreadable(error), path))?;
    let case = parse_case(&text).map_err(|kind| RunnerError::new(kind, path))?;
    check_file_name(&case, path).map_err(|kind| RunnerError::new(kind, path))?;
    Ok(case)
}

/// Parses the text of a case file and checks the rules a case must meet by itself.
///
/// # Errors
///
/// Returns the [`RunnerErrorKind`] of the first rule the case breaks.
pub fn parse_case(text: &str) -> Result<Case, RunnerErrorKind> {
    let case: Case = serde_yaml_ng::from_str(text)
        .map_err(|error| RunnerErrorKind::Malformed(error.to_string()))?;
    check_case(&case)?;
    Ok(case)
}

fn check_case(case: &Case) -> Result<(), RunnerErrorKind> {
    if case.rules.is_empty() {
        return Err(RunnerErrorKind::NoRules);
    }
    if case.expect.outcome == Outcome::Plan && case.expect.rows.is_none() {
        return Err(RunnerErrorKind::PlanWithoutRows);
    }
    if case.expect.outcome.needs_security_context() && case.security_context.is_none() {
        return Err(RunnerErrorKind::MissingSecurityContext);
    }
    Ok(())
}

fn check_file_name(case: &Case, path: &Path) -> Result<(), RunnerErrorKind> {
    let stem = path.file_stem().and_then(|stem| stem.to_str());
    if stem == Some(case.id.as_str()) {
        Ok(())
    } else {
        Err(RunnerErrorKind::IdMismatch(case.id.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAN_CASE: &str = "
id: sample-plan-001
rules: [CS-2.2]
model: sample
security_context: analyst.json
query: {contract: semantic_compiler.query}
expect:
  outcome: plan
  rows: expected.csv
  row_order: any
";

    #[test]
    fn cs_2_reads_every_field_of_a_case() {
        let case = parse_case(PLAN_CASE).unwrap();
        assert_eq!(case.id.as_str(), "sample-plan-001");
        assert_eq!(case.rules[0].as_str(), "CS-2.2");
        assert_eq!(case.expect.outcome, Outcome::Plan);
        assert_eq!(case.expect.rows.as_deref(), Some("expected.csv"));
    }

    #[test]
    fn cs_2_1_rejects_a_case_without_rules() {
        let text = PLAN_CASE.replace("rules: [CS-2.2]", "rules: []");
        assert!(matches!(parse_case(&text), Err(RunnerErrorKind::NoRules)));
    }

    #[test]
    fn cs_2_2_rejects_a_plan_case_without_rows() {
        let text = PLAN_CASE.replace("  rows: expected.csv\n", "");
        assert!(matches!(
            parse_case(&text),
            Err(RunnerErrorKind::PlanWithoutRows)
        ));
    }

    #[test]
    fn cs_2_requires_a_security_context_for_a_plan_case() {
        let text = PLAN_CASE.replace("security_context: analyst.json\n", "");
        assert!(matches!(
            parse_case(&text),
            Err(RunnerErrorKind::MissingSecurityContext)
        ));
    }

    #[test]
    fn cs_2_allows_a_load_error_case_without_security_context_or_query() {
        let text = "
id: sample-load-error-001
rules: [CS-2]
model: sample
expect: {outcome: load_error, error: E0D-02}
";
        assert_eq!(parse_case(text).unwrap().expect.outcome, Outcome::LoadError);
    }

    #[test]
    fn cs_2_rejects_an_unknown_field() {
        let text = PLAN_CASE.replace("model: sample", "model: sample\nmodle: typo");
        assert!(matches!(
            parse_case(&text),
            Err(RunnerErrorKind::Malformed(_))
        ));
    }

    #[test]
    fn cs_1_rejects_a_file_named_differently_from_its_id() {
        let case = parse_case(PLAN_CASE).unwrap();
        let result = check_file_name(&case, Path::new("cases/sample/other.yaml"));
        assert!(matches!(result, Err(RunnerErrorKind::IdMismatch(_))));
    }
}
