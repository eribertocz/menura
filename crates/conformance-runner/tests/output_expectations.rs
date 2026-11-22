//! Runs the cases of the output expectations of CS-2.10 to CS-2.16 through the runner.

use std::path::{Path, PathBuf};

use menura_conformance_runner::{CaseStatus, Report, run};

fn conformance_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

fn status<'report>(report: &'report Report, case_id: &str) -> Option<&'report CaseStatus> {
    report
        .cases
        .iter()
        .find(|case| case.id.as_str() == case_id)
        .map(|case| &case.status)
}

#[test]
fn cs_2_11_cs_2_12_cs_2_14_the_plan_output_cases_pass() {
    let report = run(&conformance_directory(), Some("cs")).unwrap();
    for case_id in ["cs-lineage-001", "cs-explanation-001", "cs-cache-key-001"] {
        assert_eq!(
            status(&report, case_id),
            Some(&CaseStatus::Passed),
            "{case_id}"
        );
    }
}

#[test]
fn cs_2_8_cases_that_need_a_missing_core_function_are_pending() {
    let report = run(&conformance_directory(), Some("next")).unwrap();
    for case_id in [
        "next-audit-001",
        "next-describe-001",
        "next-integrity-checks-001",
    ] {
        let status = status(&report, case_id);
        assert!(
            matches!(status, CaseStatus::Pending(_)),
            "{case_id}: {status:?}"
        );
    }
    assert!(!report.has_failures());
}
