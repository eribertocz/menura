//! Runs the sample cases of `conformance/cases/sample/` through the runner.

#![expect(
    clippy::disallowed_methods,
    reason = "A2: the test reads the conformance directory, like the runner itself"
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use menura_conformance_runner::case::Outcome;
use menura_conformance_runner::case_file::read_case;
use menura_conformance_runner::{CaseStatus, RunnerErrorKind, run};

fn conformance_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

#[test]
fn cs_2_sample_cases_cover_every_outcome_kind() {
    let folder = conformance_directory().join("cases/sample");
    let mut outcomes = BTreeSet::new();
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "yaml")
        {
            outcomes.insert(read_case(&path).unwrap().expect.outcome);
        }
    }
    let expected = BTreeSet::from([
        Outcome::LoadError,
        Outcome::Error,
        Outcome::Plan,
        Outcome::RuntimeError,
    ]);
    assert_eq!(outcomes, expected);
}

#[test]
fn cs_2_8_every_sample_passes_in_the_0_5_0_slice() {
    let report = run(&conformance_directory(), Some("sample")).unwrap();
    assert_eq!(report.cases.len(), 4);
    for case in &report.cases {
        assert_eq!(case.status, CaseStatus::Passed, "{}", case.id);
    }
    assert!(!report.has_failures());
}

#[test]
fn cs_4_1_running_every_area_includes_the_samples() {
    let report = run(&conformance_directory(), None).unwrap();
    assert!(
        report
            .cases
            .iter()
            .any(|case| case.id.as_str() == "sample-plan-001")
    );
}

#[test]
fn cs_4_1_unknown_area_is_an_error() {
    let error = run(&conformance_directory(), Some("no-such-area")).unwrap_err();
    assert!(matches!(error.kind, RunnerErrorKind::UnknownArea(_)));
}

#[test]
fn cs_4_3_report_lists_each_sample_case_under_its_rule() {
    let text = run(&conformance_directory(), Some("sample"))
        .unwrap()
        .render();
    assert!(text.contains("CS-2.2\n  sample-plan-001 passed"), "{text}");
    assert!(
        text.ends_with("4 cases: 4 passed, 0 pending, 0 failed\n"),
        "{text}"
    );
}
