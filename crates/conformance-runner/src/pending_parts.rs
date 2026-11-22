//! The parts of a case that need a `core` function this build does not have yet: they are
//! reported as pending, never as failed or passed (CS-2.8).

use crate::case::{Case, Outcome};
use crate::report::CaseStatus;

/// The status of a document case whose model loads and whose function `core` does not have
/// yet: the document part is pending (CS-2.15, CS-2.8). `integrity_checks` is the only such
/// function.
pub(crate) fn document_part(outcome: Outcome) -> CaseStatus {
    let function = if outcome == Outcome::Describe {
        "describe"
    } else {
        "integrity_checks"
    };
    CaseStatus::Pending(format!(
        "the document part needs `{function}`, which core does not have yet (CS-2.15, CS-2.8)"
    ))
}

/// The status of a case whose query part `status` ran: a passed part stays pending when the
/// case expects an audit record, which `core` does not return yet (CS-2.13).
pub(crate) fn with_pending_parts(case: &Case, status: CaseStatus) -> CaseStatus {
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
    CaseStatus::Passed
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
    fn cs_2_16_a_passed_plan_case_stays_passed() {
        let plan = case("plan", "  rows: e.csv\n");
        assert_eq!(
            with_pending_parts(&plan, CaseStatus::Passed),
            CaseStatus::Passed
        );
    }

    #[test]
    fn cs_2_13_a_passed_case_with_audit_is_pending() {
        let error = case("error", "  error: E2-01\n  audit: a.json\n");
        let status = with_pending_parts(&error, CaseStatus::Passed);
        assert!(is_pending(&status, "CS-2.13"), "{status:?}");
    }

    #[test]
    fn cs_2_8_a_failed_part_is_never_made_pending() {
        let plan = case("plan", "  rows: e.csv\n  audit: a.json\n");
        let failed = CaseStatus::Failed("rows differ".to_owned());
        let status = with_pending_parts(&plan, failed.clone());
        assert_eq!(status, failed);
    }

    #[test]
    fn cs_2_15_the_document_part_is_pending() {
        assert!(is_pending(&document_part(Outcome::Describe), "`describe`"));
        assert!(is_pending(
            &document_part(Outcome::IntegrityChecks),
            "`integrity_checks`"
        ));
    }
}
