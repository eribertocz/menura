//! Runs the cases of `conformance/cases/e0/` through `core` (CS-2.8): every `load_error` case
//! passes, and every other case passes or, when its query needs a capability outside the 0.3.0
//! slice, is pending (VS-4).

use std::path::{Path, PathBuf};

use menura_conformance_runner::case::Outcome;
use menura_conformance_runner::case_file::read_case;
use menura_conformance_runner::{CaseStatus, RunnerError, run};

fn conformance_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn outcome_of(case_id: &str) -> Result<Outcome, RunnerError> {
    let path = conformance_directory()
        .join("cases/e0")
        .join(format!("{case_id}.yaml"));
    Ok(read_case(&path)?.expect.outcome)
}

#[test]
fn cs_2_8_every_e0_case_passes_or_needs_a_capability_outside_the_slice() {
    let report = run(&conformance_directory(), Some("e0")).unwrap();
    let failures: Vec<String> = report
        .cases
        .iter()
        .filter_map(|case| match &case.status {
            CaseStatus::Failed(reason) => Some(format!("{}: {reason}", case.id)),
            _ => None,
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    for case in &report.cases {
        let outcome = outcome_of(case.id.as_str()).unwrap();
        match &case.status {
            CaseStatus::Pending(reason) => {
                assert_ne!(outcome, Outcome::LoadError, "{}", case.id);
                assert!(
                    reason.contains("E_NOT_YET_SUPPORTED"),
                    "{}: {reason}",
                    case.id
                );
            }
            status => assert_eq!(status, &CaseStatus::Passed, "{}", case.id),
        }
    }
}
