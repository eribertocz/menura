//! Runs the vendored Apache Ossie suites of `conformance/ossie/` through the runner (CS-3, CS-4).

#![expect(
    clippy::disallowed_methods,
    reason = "A2: the test reads the conformance directory, like the runner itself"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use semantic_compiler_conformance_runner::case::RuleId;
use semantic_compiler_conformance_runner::{CaseStatus, Report, run};

fn conformance_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}

/// The report of `just conformance ossie`, run once for every test of this file.
fn ossie_report() -> Result<&'static Report, &'static String> {
    static REPORT: OnceLock<Result<Report, String>> = OnceLock::new();
    REPORT
        .get_or_init(|| run(&conformance_directory(), Some("ossie")).map_err(|e| e.to_string()))
        .as_ref()
}

fn case_folders(suite: &str) -> usize {
    let folder = conformance_directory()
        .join("ossie")
        .join(suite)
        .join("cases");
    fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .count()
}

#[test]
fn oss_2_3_every_vendored_case_is_reported_pending() {
    let report = ossie_report().unwrap();
    let expected = case_folders("wide_table") + case_folders("query");
    assert_eq!(report.cases.len(), expected);
    for case in &report.cases {
        assert!(
            matches!(case.status, CaseStatus::Pending(_)),
            "{} is {:?}",
            case.id,
            case.status
        );
    }
}

#[test]
fn cs_4_4_wide_table_cases_are_reported_under_their_rules() {
    let report = ossie_report().unwrap();
    let case = report
        .cases
        .iter()
        .find(|case| case.id.as_str() == "wide_table/e22_ambiguous_path_dimension")
        .unwrap();
    let rules: Vec<&str> = case.rules.iter().map(RuleId::as_str).collect();
    assert_eq!(rules, ["CS-4.4", "OSS-2.3"]);
}

#[test]
fn cs_4_6_query_cases_are_reported_under_their_rules() {
    let report = ossie_report().unwrap();
    let case = report
        .cases
        .iter()
        .find(|case| case.id.as_str() == "query/a01_grand_total")
        .unwrap();
    let rules: Vec<&str> = case.rules.iter().map(RuleId::as_str).collect();
    assert_eq!(rules, ["CS-4.6", "OSS-2.3"]);
}

#[test]
fn cs_4_1_a_full_run_includes_the_ossie_suites() {
    let report = run(&conformance_directory(), None).unwrap();
    assert!(
        report
            .cases
            .iter()
            .any(|case| case.id.as_str() == "query/b20_having_on_one_side_measure")
    );
}

#[test]
fn cs_3_1_the_suites_are_vendored_at_the_tracked_commit() {
    let text = fs::read_to_string(conformance_directory().join("ossie/upstream.json")).unwrap();
    let upstream: serde_json::Value = serde_json::from_str(&text).unwrap();
    let commit = upstream["tracked_commit"].as_str().unwrap();
    assert!(commit.starts_with("aa500d3"));
    assert_eq!(commit.len(), 40);
}

#[test]
fn cs_4_5_the_runner_context_is_the_one_of_the_specification() {
    let text =
        fs::read_to_string(conformance_directory().join("ossie/runner-context.json")).unwrap();
    let context: serde_json::Value = serde_json::from_str(&text).unwrap();
    let expected = serde_json::json!({
        "contract": "semantic_compiler.security_context",
        "contract_version": "1.0.0",
        "subject": "conformance-runner",
        "roles": [],
        "attributes": {}
    });
    assert_eq!(context, expected);
}
