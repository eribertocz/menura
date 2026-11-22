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
fn cs_2_15_the_integrity_checks_document_matches_its_golden_file() {
    let report = run(&conformance_directory(), Some("cs")).unwrap();
    assert_eq!(
        status(&report, "cs-integrity-checks-001"),
        Some(&CaseStatus::Passed)
    );
}

#[test]
fn cs_2_15_the_describe_cases_pass() {
    let report = run(&conformance_directory(), Some("cs")).unwrap();
    assert_eq!(
        status(&report, "cs-describe-001"),
        Some(&CaseStatus::Passed)
    );
    let report = run(&conformance_directory(), Some("dsc")).unwrap();
    for case_id in [
        "dsc-describe-admin-001",
        "dsc-describe-analyst-001",
        "dsc-describe-not-admitted-001",
    ] {
        assert_eq!(
            status(&report, case_id),
            Some(&CaseStatus::Passed),
            "{case_id}"
        );
    }
}

#[test]
fn cs_2_13_the_audit_cases_pass() {
    let report = run(&conformance_directory(), Some("pln")).unwrap();
    for case_id in [
        "pln-audit-plan-001",
        "pln-audit-row-policies-001",
        "pln-audit-not-admitted-001",
        "pln-audit-denied-001",
        "pln-audit-denied-field-001",
    ] {
        assert_eq!(
            status(&report, case_id),
            Some(&CaseStatus::Passed),
            "{case_id}"
        );
    }
}
