//! Runs the vendored Apache Ossie suites of `conformance/ossie/` through the runner (CS-3, CS-4).

#![expect(
    clippy::disallowed_methods,
    reason = "A2: the test reads the conformance directory, like the runner itself"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use menura_conformance_runner::case::RuleId;
use menura_conformance_runner::ossie::check_layout;
use menura_conformance_runner::{CaseStatus, Report, run};

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

/// The Layer 2 cases the 0.5.0 slice compiles (issue #382), which must pass (CS-4.6).
const PASSING_QUERY_CASES: [&str; 7] = [
    "query/a01_grand_total",
    "query/a02_measure_per_group",
    "query/a03_where_before_measure",
    "query/a05_scalar_ops_on_measure_values",
    "query/a06_nulls_follow_aggregate",
    "query/a07_empty_input_set",
    "query/a08_group_by_all_skips_measures",
];

#[test]
fn oss_2_3_every_vendored_case_is_reported() {
    let report = ossie_report().unwrap();
    let expected = case_folders("wide_table") + case_folders("query");
    assert_eq!(report.cases.len(), expected);
    assert!(!report.has_failures(), "{}", report.render());
}

#[test]
fn cs_4_6_the_query_cases_the_slice_compiles_pass() {
    let report = ossie_report().unwrap();
    for id in PASSING_QUERY_CASES {
        let case = report
            .cases
            .iter()
            .find(|case| case.id.as_str() == id)
            .unwrap();
        assert_eq!(case.status, CaseStatus::Passed, "{id}");
    }
}

#[test]
fn oss_2_4_every_other_query_case_is_reported_pending() {
    let report = ossie_report().unwrap();
    for case in &report.cases {
        let id = case.id.as_str();
        if PASSING_QUERY_CASES.contains(&id) || id.starts_with("wide_table/") {
            continue;
        }
        assert!(
            matches!(case.status, CaseStatus::Pending(_)),
            "{} is {:?}",
            case.id,
            case.status
        );
    }
}

#[test]
fn oss_2_4b_every_layer_3_case_is_run_and_compared() {
    let report = ossie_report().unwrap();
    let wide_table: Vec<_> = report
        .cases
        .iter()
        .filter(|case| case.id.as_str().starts_with("wide_table/"))
        .collect();
    assert_eq!(wide_table.len(), case_folders("wide_table"));
    for case in wide_table {
        match &case.status {
            CaseStatus::Passed => {}
            CaseStatus::Pending(reason) => assert!(
                reason.contains("(OSS-2.4): "),
                "{} is pending without a comparison: {reason}",
                case.id
            ),
            CaseStatus::Failed(reason) => panic!("{} fails: {reason}", case.id),
        }
    }
}

#[test]
fn oss_2_4a_the_layer_3_cases_that_pass_are_no_longer_listed() {
    let report = ossie_report().unwrap();
    assert!(
        report
            .cases
            .iter()
            .filter(|case| case.id.as_str().starts_with("wide_table/"))
            .any(|case| case.status == CaseStatus::Passed),
        "no Layer 3 case passes"
    );
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
fn cs_4_1_a_run_without_area_does_not_judge_the_ossie_suites() {
    let report = run(&conformance_directory(), None).unwrap();
    assert!(!report.cases.is_empty());
    assert!(
        report
            .cases
            .iter()
            .all(|case| !case.rules.iter().any(|rule| rule.as_str() == "OSS-2.3"))
    );
}

#[test]
fn cs_4_1_the_layout_of_the_ossie_folder_is_valid() {
    check_layout(&conformance_directory().join("ossie")).unwrap();
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
        "contract": "menura.security_context",
        "contract_version": "1.1.0",
        "subject": "conformance-runner",
        "roles": [],
        "attributes": {}
    });
    assert_eq!(context, expected);
}
