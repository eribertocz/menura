//! Runs the cases of `conformance/cases/e0/` through `core` (CS-2.8): every `load_error` case
//! passes, and the query part of every other case stays pending.

use std::path::{Path, PathBuf};

use semantic_compiler_conformance_runner::case::Outcome;
use semantic_compiler_conformance_runner::case_file::read_case;
use semantic_compiler_conformance_runner::runner::QUERY_PENDING_REASON;
use semantic_compiler_conformance_runner::{CaseStatus, run};

fn conformance_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn outcome_of(case_id: &str) -> Outcome {
    let path = conformance_directory()
        .join("cases/e0")
        .join(format!("{case_id}.yaml"));
    read_case(&path).unwrap().expect.outcome
}

#[test]
fn cs_2_8_the_load_part_of_every_e0_case_passes() {
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
        let expected = if outcome_of(case.id.as_str()) == Outcome::LoadError {
            CaseStatus::Passed
        } else {
            CaseStatus::Pending(QUERY_PENDING_REASON.to_owned())
        };
        assert_eq!(case.status, expected, "{}", case.id);
    }
}
