//! The rules of CS-2.11 to CS-2.15, CS-2.17 and CS-2.18 on which outcomes allow which output
//! expectations and compile options.

use crate::case::{Case, Outcome};
use crate::error::RunnerErrorKind;

/// Checks that every output expectation of `case` is allowed with its outcome, and that a
/// `describe` or `integrity_checks` case names its golden `document` (CS-2.11 to CS-2.15,
/// CS-2.17, CS-2.18).
///
/// # Errors
///
/// [`RunnerErrorKind::ExpectationNotAllowed`] naming the first member the outcome does not
/// allow, or [`RunnerErrorKind::MissingDocument`].
pub(crate) fn check_output_expectations(case: &Case) -> Result<(), RunnerErrorKind> {
    let outcome = case.expect.outcome;
    if outcome != Outcome::Error {
        refuse_present(&[(
            "alternatives",
            "CS-2.17",
            case.expect.alternatives.is_some(),
        )])?;
    }
    if !matches!(outcome, Outcome::Plan | Outcome::Error) {
        refuse_present(&[("compile_options", "CS-2.18", case.compile_options.is_some())])?;
    }
    if outcome.returns_document() {
        return check_document_case(case);
    }
    let expect = &case.expect;
    let plan_only = [
        ("lineage", "CS-2.11", expect.lineage.is_some()),
        ("explanation", "CS-2.12", expect.explanation.is_some()),
        ("cache_key", "CS-2.14", expect.cache_key.is_some()),
    ];
    if outcome != Outcome::Plan {
        refuse_present(&plan_only)?;
    }
    if !matches!(outcome, Outcome::Plan | Outcome::Error) {
        refuse_present(&[("audit", "CS-2.13", expect.audit.is_some())])?;
    }
    refuse_present(&[("document", "CS-2.15", expect.document.is_some())])
}

/// A `describe` or `integrity_checks` case requires `document` and forbids the members of
/// query outcomes (CS-2.15).
fn check_document_case(case: &Case) -> Result<(), RunnerErrorKind> {
    let expect = &case.expect;
    let forbidden = [
        ("query", case.query.is_some()),
        ("error", expect.error.is_some()),
        ("location", expect.location.is_some()),
        ("sql", expect.sql.is_some()),
        ("rows", expect.rows.is_some()),
        ("row_order", expect.row_order.is_some()),
        ("tolerance", expect.tolerance.is_some()),
        ("lineage", expect.lineage.is_some()),
        ("explanation", expect.explanation.is_some()),
        ("audit", expect.audit.is_some()),
        ("cache_key", expect.cache_key.is_some()),
    ];
    let forbidden = forbidden.map(|(member, present)| (member, "CS-2.15", present));
    refuse_present(&forbidden)?;
    if expect.document.is_none() {
        return Err(RunnerErrorKind::MissingDocument);
    }
    Ok(())
}

/// Fails on the first `(member, rule, present)` whose member is present.
fn refuse_present(members: &[(&'static str, &'static str, bool)]) -> Result<(), RunnerErrorKind> {
    match members.iter().find(|(_, _, present)| *present) {
        Some(&(member, rule, _)) => Err(RunnerErrorKind::ExpectationNotAllowed { member, rule }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::case_file::parse_case;
    use crate::error::RunnerErrorKind;

    const PLAN_CASE: &str = "
id: cs-unit-001
rules: [CS-2.11]
model: slice
security_context: auditor.json
query: {contract: menura.query}
expect:
  outcome: plan
  rows: expected.csv
";

    const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn with_outcome(outcome: &str, members: &str) -> String {
        PLAN_CASE.replace(
            "  outcome: plan\n  rows: expected.csv\n",
            &format!("  outcome: {outcome}\n{members}"),
        )
    }

    fn refused(text: &str) -> Option<(&'static str, &'static str)> {
        match parse_case(text) {
            Err(RunnerErrorKind::ExpectationNotAllowed { member, rule }) => Some((member, rule)),
            Ok(_) => None,
            Err(other) => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn cs_2_11_to_2_14_a_plan_case_reads_every_plan_expectation() {
        let text = PLAN_CASE.replace(
            "  rows: expected.csv\n",
            &format!(
                "  rows: expected.csv\n  lineage: l.json\n  explanation: [E5-2.2, PLN-7.1]\n  \
                 audit: a.json\n  cache_key: \"{HASH}\"\n"
            ),
        );
        let expect = parse_case(&text).unwrap().expect;
        assert_eq!(expect.lineage.as_deref(), Some("l.json"));
        assert_eq!(
            expect.explanation,
            Some(vec!["E5-2.2".to_owned(), "PLN-7.1".to_owned()])
        );
        assert_eq!(expect.audit.as_deref(), Some("a.json"));
        assert_eq!(expect.cache_key.unwrap().as_str(), HASH);
    }

    #[test]
    fn cs_2_12_reads_an_empty_explanation() {
        let text = PLAN_CASE.replace(
            "  rows: expected.csv\n",
            "  rows: expected.csv\n  explanation: []\n",
        );
        assert_eq!(parse_case(&text).unwrap().expect.explanation, Some(vec![]));
    }

    #[test]
    fn cs_2_14_rejects_a_cache_key_that_is_not_64_lowercase_hex_digits() {
        let text = PLAN_CASE.replace(
            "  rows: expected.csv\n",
            &format!(
                "  rows: expected.csv\n  cache_key: \"{}\"\n",
                HASH.to_uppercase()
            ),
        );
        assert!(matches!(
            parse_case(&text),
            Err(RunnerErrorKind::Malformed(_))
        ));
    }

    #[test]
    fn cs_2_11_lineage_is_allowed_only_with_a_plan() {
        let text = with_outcome("error", "  error: E2-01\n  lineage: l.json\n");
        assert_eq!(refused(&text), Some(("lineage", "CS-2.11")));
    }

    #[test]
    fn cs_2_12_explanation_is_allowed_only_with_a_plan() {
        let text = with_outcome("runtime_error", "  error: RADD-01\n  explanation: []\n");
        assert_eq!(refused(&text), Some(("explanation", "CS-2.12")));
    }

    #[test]
    fn cs_2_13_audit_is_allowed_with_an_error_or_a_plan() {
        let error = with_outcome("error", "  error: E2-01\n  audit: a.json\n");
        assert_eq!(refused(&error), None);
        let runtime = with_outcome("runtime_error", "  error: RADD-01\n  audit: a.json\n");
        assert_eq!(refused(&runtime), Some(("audit", "CS-2.13")));
        let load = "id: cs-unit-001\nrules: [CS-2.13]\nmodel: slice\n\
                    expect: {outcome: load_error, error: E0D-02, audit: a.json}\n";
        assert_eq!(refused(load), Some(("audit", "CS-2.13")));
    }

    #[test]
    fn cs_2_14_cache_key_is_allowed_only_with_a_plan() {
        let text = with_outcome(
            "error",
            &format!("  error: E2-01\n  cache_key: \"{HASH}\"\n"),
        );
        assert_eq!(refused(&text), Some(("cache_key", "CS-2.14")));
    }

    #[test]
    fn cs_2_17_reads_the_alternatives_of_an_error_case() {
        let text = with_outcome(
            "error",
            "  error: E_NAME_NOT_FOUND\n  alternatives: [\"metric:BILLED\"]\n",
        );
        let expect = parse_case(&text).unwrap().expect;
        assert_eq!(expect.alternatives, Some(vec!["metric:BILLED".to_owned()]));
    }

    #[test]
    fn cs_2_17_alternatives_are_allowed_only_with_an_error() {
        let plan = PLAN_CASE.replace(
            "  rows: expected.csv\n",
            "  rows: expected.csv\n  alternatives: []\n",
        );
        assert_eq!(refused(&plan), Some(("alternatives", "CS-2.17")));
        let runtime = with_outcome("runtime_error", "  error: RADD-01\n  alternatives: []\n");
        assert_eq!(refused(&runtime), Some(("alternatives", "CS-2.17")));
        let describe = "id: cs-unit-001\nrules: [CS-2.17]\nmodel: slice\n\
                        security_context: auditor.json\n\
                        expect: {outcome: describe, document: d.json, alternatives: []}\n";
        assert_eq!(refused(describe), Some(("alternatives", "CS-2.17")));
    }

    #[test]
    fn cs_2_18_reads_an_actor_of_any_json_value() {
        let text = PLAN_CASE.replace(
            "expect:\n",
            "compile_options: {actor: {id: agent-7}}\nexpect:\n",
        );
        let options = parse_case(&text).unwrap().compile_options.unwrap();
        assert_eq!(options.actor, Some(serde_json::json!({"id": "agent-7"})));
    }

    #[test]
    fn cs_2_18_compile_options_hold_only_actor() {
        let text = PLAN_CASE.replace("expect:\n", "compile_options: {audit: true}\nexpect:\n");
        assert!(matches!(
            parse_case(&text),
            Err(RunnerErrorKind::Malformed(_))
        ));
    }

    #[test]
    fn cs_2_18_compile_options_are_allowed_only_with_an_error_or_a_plan() {
        let options = "compile_options: {actor: agent-7}\n";
        let error = with_outcome("error", "  error: E2-01\n")
            .replace("expect:\n", &format!("{options}expect:\n"));
        assert_eq!(refused(&error), None);
        let runtime = with_outcome("runtime_error", "  error: RADD-01\n")
            .replace("expect:\n", &format!("{options}expect:\n"));
        assert_eq!(refused(&runtime), Some(("compile_options", "CS-2.18")));
        let describe = format!(
            "id: cs-unit-001\nrules: [CS-2.18]\nmodel: slice\nsecurity_context: auditor.json\n\
             {options}expect: {{outcome: describe, document: d.json}}\n"
        );
        assert_eq!(refused(&describe), Some(("compile_options", "CS-2.18")));
    }

    #[test]
    fn cs_2_15_document_is_allowed_only_with_describe_and_integrity_checks() {
        let text = PLAN_CASE.replace(
            "  rows: expected.csv\n",
            "  rows: expected.csv\n  document: d.json\n",
        );
        assert_eq!(refused(&text), Some(("document", "CS-2.15")));
    }

    #[test]
    fn cs_2_15_reads_a_describe_case() {
        let text = "id: cs-unit-001\nrules: [CS-2.15]\nmodel: slice\n\
                    security_context: auditor.json\n\
                    expect: {outcome: describe, document: d.json}\n";
        let case = parse_case(text).unwrap();
        assert_eq!(case.expect.document.as_deref(), Some("d.json"));
    }

    #[test]
    fn cs_2_15_an_integrity_checks_case_needs_no_security_context() {
        let text = "id: cs-unit-001\nrules: [CS-2.15]\nmodel: slice\n\
                    expect: {outcome: integrity_checks, document: d.json}\n";
        assert!(parse_case(text).is_ok());
    }

    #[test]
    fn cs_2_15_a_describe_case_needs_a_security_context() {
        let text = "id: cs-unit-001\nrules: [CS-2.15]\nmodel: slice\n\
                    expect: {outcome: describe, document: d.json}\n";
        assert!(matches!(
            parse_case(text),
            Err(RunnerErrorKind::MissingSecurityContext)
        ));
    }

    #[test]
    fn cs_2_15_describe_and_integrity_checks_require_document() {
        for outcome in ["describe", "integrity_checks"] {
            let text = format!(
                "id: cs-unit-001\nrules: [CS-2.15]\nmodel: slice\n\
                 security_context: auditor.json\nexpect: {{outcome: {outcome}}}\n"
            );
            assert!(matches!(
                parse_case(&text),
                Err(RunnerErrorKind::MissingDocument)
            ));
        }
    }

    #[test]
    fn cs_2_15_describe_and_integrity_checks_forbid_the_query_members() {
        let cache_key = format!(", cache_key: \"{HASH}\"");
        let members = [
            ("query", "query: {contract: menura.query}\n", ""),
            ("error", "", ", error: E2-01"),
            ("location", "", ", location: {document: model, pointer: ''}"),
            ("sql", "", ", sql: e.sql"),
            ("rows", "", ", rows: e.csv"),
            ("row_order", "", ", row_order: any"),
            ("tolerance", "", ", tolerance: 0.1"),
            ("lineage", "", ", lineage: l.json"),
            ("explanation", "", ", explanation: []"),
            ("audit", "", ", audit: a.json"),
            ("cache_key", "", cache_key.as_str()),
        ];
        for outcome in ["describe", "integrity_checks"] {
            for (member, top, inside) in &members {
                let text = format!(
                    "id: cs-unit-001\nrules: [CS-2.15]\nmodel: slice\n\
                     security_context: auditor.json\n{top}\
                     expect: {{outcome: {outcome}, document: d.json{inside}}}\n"
                );
                assert_eq!(
                    refused(&text),
                    Some((*member, "CS-2.15")),
                    "{outcome} {member}"
                );
            }
        }
    }
}
