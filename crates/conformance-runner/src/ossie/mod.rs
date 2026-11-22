//! The vendored Apache Ossie suites of `conformance/ossie/` (CS-3): finds their cases, prepares
//! each one as CS-4.4 and CS-4.6 say, runs the Layer 2 cases (CS-4.6), and checks the results
//! against the expected-failure list (OSS-2.4).

mod catalog_schema;
mod comparison;
pub mod created_tables;
pub mod expected;
pub mod expected_failures;
mod outer_order;
pub mod prepare;
mod query_run;
pub mod suite;

use std::collections::BTreeSet;
use std::path::Path;

use menura_contracts::IncomingContract;
use serde_json::Value;

use crate::case::CaseId;
use crate::error::{RunnerError, RunnerErrorKind};
use crate::fixture::read_text;
use crate::report::{CaseResult, CaseStatus};
use expected_failures::{
    ExpectedFailure, ExpectedFailures, check_entries, parse_expected_failures,
};
use prepare::prepare_case;
use query_run::query_mismatch;
use suite::{OssieCase, Suite, discover_cases};

/// The area name that selects the Apache Ossie suites (`just conformance ossie`).
pub const OSSIE_AREA: &str = "ossie";

/// What running a case gave.
#[derive(Debug)]
enum CaseOutcome {
    /// The case files or its fixture cannot be prepared.
    Unprepared(RunnerError),
    /// The case is prepared, but the runner does not run its suite yet (the wide-table form).
    NotRun,
    /// The case ran and does not match its expectation, with the reason.
    Mismatch(String),
    /// The case ran and matches its expectation.
    Matched,
}

/// What the layout of `conformance/ossie/` holds once it is checked.
struct Suites {
    failures: ExpectedFailures,
    security_context: Value,
    cases: Vec<OssieCase>,
}

/// Checks the layout of `ossie_directory` without running any suite case (CS-4.1): the
/// expected-failure list is well formed and every entry names a vendored case (OSS-2.4), and the
/// runner security context is valid (CS-4.5).
///
/// # Errors
///
/// Returns a [`RunnerError`] when the suites cannot be read, `expected-failures.yaml` is malformed
/// or names a case that is not vendored, or `runner-context.json` is not a valid security context.
pub fn check_layout(ossie_directory: &Path) -> Result<(), RunnerError> {
    read_suites(ossie_directory).map(|_| ())
}

/// Runs every case of the suites vendored in `ossie_directory` (`just conformance ossie`,
/// OSS-2.3), after checking their layout as [`check_layout`] does.
///
/// A Layer 2 case is run through `core` and compared with its expected result (CS-4.6); a
/// wide-table case is only prepared. A case that does not pass is pending when it is listed in
/// `expected-failures.yaml` and fails when it is not; a case whose fixtures cannot be prepared
/// fails (OSS-2.4).
///
/// # Errors
///
/// Returns the errors of [`check_layout`].
pub fn run_suites(ossie_directory: &Path) -> Result<Vec<CaseResult>, RunnerError> {
    let suites = read_suites(ossie_directory)?;
    Ok(suites
        .cases
        .iter()
        .map(|case| {
            let outcome = run_case(case, &suites.security_context);
            CaseResult {
                id: CaseId::new(case.key()),
                rules: case.suite.rules(),
                status: case_status(outcome, suites.failures.get(&case.key())),
            }
        })
        .collect())
}

/// Reads and checks the layout of `ossie_directory`.
fn read_suites(ossie_directory: &Path) -> Result<Suites, RunnerError> {
    let failures_path = ossie_directory.join("expected-failures.yaml");
    let failures = parse_expected_failures(&read_text(&failures_path)?)
        .map_err(|kind| RunnerError::new(kind, &failures_path))?;
    let security_context = read_runner_context(&ossie_directory.join("runner-context.json"))?;
    let cases = discover_cases(ossie_directory)?;
    let keys: BTreeSet<String> = cases.iter().map(OssieCase::key).collect();
    check_entries(&failures, &keys).map_err(|kind| RunnerError::new(kind, &failures_path))?;
    Ok(Suites {
        failures,
        security_context,
        cases,
    })
}

/// Prepares `case` and, for a Layer 2 case, runs it.
fn run_case(case: &OssieCase, security_context: &Value) -> CaseOutcome {
    match prepare_case(case, security_context) {
        Err(error) => CaseOutcome::Unprepared(error),
        Ok(_) if case.suite == Suite::WideTable => CaseOutcome::NotRun,
        Ok((prepared, connection)) => match query_mismatch(&case.key(), &prepared, &connection) {
            Some(reason) => CaseOutcome::Mismatch(reason),
            None => CaseOutcome::Matched,
        },
    }
}

/// The status of a case that gave `outcome`, listed with `entry`. No suite blocks yet
/// (OSS-2.3), so a case that passes while listed is pending, never failed (OSS-2.4).
fn case_status(outcome: CaseOutcome, entry: Option<&ExpectedFailure>) -> CaseStatus {
    const NOT_LISTED: &str = "does not pass and is not listed in expected-failures.yaml (OSS-2.4)";
    match (outcome, entry) {
        (CaseOutcome::Unprepared(error), _) => CaseStatus::Failed(error.to_string()),
        (CaseOutcome::Matched, None) => CaseStatus::Passed,
        (CaseOutcome::Matched, Some(_)) => CaseStatus::Pending(
            "passes while listed in expected-failures.yaml; remove its entry (OSS-2.4)".to_owned(),
        ),
        (CaseOutcome::NotRun, Some(entry)) => CaseStatus::Pending(listed_reason(entry)),
        (CaseOutcome::Mismatch(reason), Some(entry)) => {
            CaseStatus::Pending(format!("{}: {reason}", listed_reason(entry)))
        }
        (CaseOutcome::NotRun, None) => CaseStatus::Failed(NOT_LISTED.to_owned()),
        (CaseOutcome::Mismatch(reason), None) => {
            CaseStatus::Failed(format!("{reason}; the case {NOT_LISTED}"))
        }
    }
}

fn listed_reason(entry: &ExpectedFailure) -> String {
    format!(
        "{}, expected to pass in {} (OSS-2.4)",
        entry.reason, entry.fixed_in
    )
}

/// Reads `runner-context.json` and admits it as a security context (CS-4.5).
fn read_runner_context(path: &Path) -> Result<Value, RunnerError> {
    let contract = IncomingContract::SecurityContext;
    let invalid = |error: menura_contracts::AdmissionError| {
        RunnerError::new(
            RunnerErrorKind::InvalidRunnerContext(error.to_string()),
            path,
        )
    };
    let document = contract.parse_json(&read_text(path)?).map_err(invalid)?;
    contract.check_contract(&document).map_err(invalid)?;
    contract.validate(&document).map_err(invalid)?;
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> ExpectedFailure {
        ExpectedFailure {
            reason: "not implemented".to_owned(),
            fixed_in: "0.7.0".to_owned(),
            issue: None,
        }
    }

    #[test]
    fn oss_2_4_a_listed_case_is_pending_with_its_reason() {
        assert_eq!(
            case_status(CaseOutcome::NotRun, Some(&entry())),
            CaseStatus::Pending("not implemented, expected to pass in 0.7.0 (OSS-2.4)".to_owned())
        );
        assert_eq!(
            case_status(CaseOutcome::Mismatch("no".to_owned()), Some(&entry())),
            CaseStatus::Pending(
                "not implemented, expected to pass in 0.7.0 (OSS-2.4): no".to_owned()
            )
        );
    }

    #[test]
    fn oss_2_4_a_case_that_does_not_pass_and_is_not_listed_fails() {
        assert!(matches!(
            case_status(CaseOutcome::NotRun, None),
            CaseStatus::Failed(_)
        ));
        assert!(matches!(
            case_status(CaseOutcome::Mismatch("no".to_owned()), None),
            CaseStatus::Failed(_)
        ));
    }

    #[test]
    fn cs_4_6_a_case_that_matches_and_is_not_listed_passes() {
        assert_eq!(case_status(CaseOutcome::Matched, None), CaseStatus::Passed);
    }

    #[test]
    fn oss_2_4_a_case_that_passes_while_listed_is_pending_while_no_suite_blocks() {
        assert!(matches!(
            case_status(CaseOutcome::Matched, Some(&entry())),
            CaseStatus::Pending(_)
        ));
    }

    #[test]
    fn cs_4_4_a_case_whose_fixture_cannot_be_prepared_fails() {
        let error = RunnerError::new(RunnerErrorKind::NoRules, "x");
        assert!(matches!(
            case_status(CaseOutcome::Unprepared(error), Some(&entry())),
            CaseStatus::Failed(_)
        ));
    }
}
