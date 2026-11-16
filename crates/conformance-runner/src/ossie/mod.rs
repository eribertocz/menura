//! The vendored Apache Ossie suites of `conformance/ossie/` (CS-3): finds their cases, prepares
//! each one as CS-4.4 and CS-4.6 say, and checks the results against the expected-failure list
//! (OSS-2.4).

pub mod created_tables;
pub mod expected;
pub mod expected_failures;
pub mod prepare;
pub mod suite;

use std::collections::BTreeSet;
use std::path::Path;

use semantic_compiler_contracts::IncomingContract;
use serde_json::Value;

use crate::case::CaseId;
use crate::error::{RunnerError, RunnerErrorKind};
use crate::fixture::read_text;
use crate::report::{CaseResult, CaseStatus};
use expected_failures::{ExpectedFailure, check_entries, parse_expected_failures};
use prepare::prepare_case;
use suite::discover_cases;

/// The area name that selects the Apache Ossie suites (`just conformance ossie`).
pub const OSSIE_AREA: &str = "ossie";

/// Runs every case of the suites vendored in `ossie_directory`.
///
/// While no compilation stage exists, a case is pending when its fixtures can be prepared and it
/// is listed in `expected-failures.yaml`; it fails when its fixtures cannot be prepared or it is
/// not listed (OSS-2.4).
///
/// # Errors
///
/// Returns a [`RunnerError`] when the suites cannot be read, `expected-failures.yaml` is malformed
/// or names a case that is not vendored, or `runner-context.json` is not a valid security context.
pub fn run_suites(ossie_directory: &Path) -> Result<Vec<CaseResult>, RunnerError> {
    let failures_path = ossie_directory.join("expected-failures.yaml");
    let failures = parse_expected_failures(&read_text(&failures_path)?)
        .map_err(|kind| RunnerError::new(kind, &failures_path))?;
    let security_context = read_runner_context(&ossie_directory.join("runner-context.json"))?;
    let cases = discover_cases(ossie_directory)?;
    let keys: BTreeSet<String> = cases.iter().map(suite::OssieCase::key).collect();
    check_entries(&failures, &keys).map_err(|kind| RunnerError::new(kind, &failures_path))?;
    Ok(cases
        .iter()
        .map(|case| {
            let prepared = prepare_case(case, &security_context).map(|_| ());
            CaseResult {
                id: CaseId::new(case.key()),
                rules: case.suite.rules(),
                status: case_status(prepared, failures.get(&case.key())),
            }
        })
        .collect())
}

/// The status of a case whose preparation gave `prepared`, listed with `entry`.
fn case_status(prepared: Result<(), RunnerError>, entry: Option<&ExpectedFailure>) -> CaseStatus {
    match (prepared, entry) {
        (Err(error), _) => CaseStatus::Failed(error.to_string()),
        (Ok(()), Some(entry)) => CaseStatus::Pending(format!(
            "{}, expected to pass in {} (OSS-2.4)",
            entry.reason, entry.fixed_in
        )),
        (Ok(()), None) => CaseStatus::Failed(
            "does not pass and is not listed in expected-failures.yaml (OSS-2.4)".to_owned(),
        ),
    }
}

/// Reads `runner-context.json` and admits it as a security context (CS-4.5).
fn read_runner_context(path: &Path) -> Result<Value, RunnerError> {
    let contract = IncomingContract::SecurityContext;
    let invalid = |error: semantic_compiler_contracts::AdmissionError| {
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
        }
    }

    #[test]
    fn oss_2_4_a_listed_case_is_pending_with_its_reason() {
        assert_eq!(
            case_status(Ok(()), Some(&entry())),
            CaseStatus::Pending("not implemented, expected to pass in 0.7.0 (OSS-2.4)".to_owned())
        );
    }

    #[test]
    fn oss_2_4_a_case_that_does_not_pass_and_is_not_listed_fails() {
        assert!(matches!(case_status(Ok(()), None), CaseStatus::Failed(_)));
    }

    #[test]
    fn cs_4_4_a_case_whose_fixture_cannot_be_prepared_fails() {
        let error = RunnerError::new(RunnerErrorKind::NoRules, "x");
        assert!(matches!(
            case_status(Err(error), Some(&entry())),
            CaseStatus::Failed(_)
        ));
    }
}
