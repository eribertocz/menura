//! The parts of a case that need a `core` function this build does not have yet: they are
//! reported as pending, never as failed or passed (CS-2.8).

use menura_contracts::ContractVersion;

use crate::case::{Case, Outcome};
use crate::report::CaseStatus;

/// The version of the build under test: the workspace version.
const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The version from which every `outcome: plan` case also runs `explain` (CS-2.16).
const EXPLAIN_FROM: ContractVersion = ContractVersion {
    major: 0,
    minor: 7,
    patch: 0,
};

/// The status of a case whose query part `status` ran: a passed part stays pending when the
/// case expects an audit record, which `core` does not return yet (CS-2.13), or when it is an
/// `outcome: plan` case of a version that runs `explain`, which `core` does not have yet
/// (CS-2.16).
pub(crate) fn with_pending_parts(case: &Case, status: CaseStatus) -> CaseStatus {
    with_pending_parts_of_version(case, status, BUILD_VERSION)
}

fn with_pending_parts_of_version(case: &Case, status: CaseStatus, version: &str) -> CaseStatus {
    if status != CaseStatus::Passed {
        return status;
    }
    if case.expect.audit.is_some() {
        return CaseStatus::Pending(
            "the audit part needs the audit record, which core does not return yet \
             (CS-2.13, CS-2.8)"
                .to_owned(),
        );
    }
    if case.expect.outcome == Outcome::Plan && runs_explain(version) {
        return CaseStatus::Pending(
            "the explain part needs `explain`, which core does not have yet (CS-2.16, CS-2.8)"
                .to_owned(),
        );
    }
    CaseStatus::Passed
}

/// Whether the build of `version` runs `explain` for every `outcome: plan` case (CS-2.16).
fn runs_explain(version: &str) -> bool {
    ContractVersion::parse(version).is_some_and(|version| version >= EXPLAIN_FROM)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case_file::parse_case;

    fn case(outcome: &str, members: &str) -> Case {
        let text = format!(
            "id: cs-unit-001\nrules: [CS-2.8]\nmodel: slice\nsecurity_context: auditor.json\n\
             query: {{contract: menura.query}}\nexpect:\n  outcome: {outcome}\n{members}"
        );
        parse_case(&text).unwrap()
    }

    fn is_pending(status: &CaseStatus, words: &str) -> bool {
        matches!(status, CaseStatus::Pending(reason) if reason.contains(words))
    }

    #[test]
    fn cs_2_16_explain_runs_from_0_7_0() {
        assert!(!runs_explain("0.6.0"));
        assert!(runs_explain("0.7.0"));
        assert!(runs_explain("1.0.0"));
    }

    #[test]
    fn cs_2_16_a_passed_plan_case_is_pending_while_core_has_no_explain() {
        let plan = case("plan", "  rows: e.csv\n");
        let status = with_pending_parts_of_version(&plan, CaseStatus::Passed, "0.7.0");
        assert!(is_pending(&status, "CS-2.16"), "{status:?}");
        let before = with_pending_parts_of_version(&plan, CaseStatus::Passed, "0.6.0");
        assert_eq!(before, CaseStatus::Passed);
    }

    #[test]
    fn cs_2_16_explain_does_not_apply_to_other_outcomes() {
        let error = case("error", "  error: E2-01\n");
        let status = with_pending_parts_of_version(&error, CaseStatus::Passed, "0.7.0");
        assert_eq!(status, CaseStatus::Passed);
    }

    #[test]
    fn cs_2_13_a_passed_case_with_audit_is_pending() {
        let error = case("error", "  error: E2-01\n  audit: a.json\n");
        let status = with_pending_parts_of_version(&error, CaseStatus::Passed, "0.6.0");
        assert!(is_pending(&status, "CS-2.13"), "{status:?}");
    }

    #[test]
    fn cs_2_8_a_failed_part_is_never_made_pending() {
        let plan = case("plan", "  rows: e.csv\n  audit: a.json\n");
        let failed = CaseStatus::Failed("rows differ".to_owned());
        let status = with_pending_parts_of_version(&plan, failed.clone(), "0.7.0");
        assert_eq!(status, failed);
    }
}
