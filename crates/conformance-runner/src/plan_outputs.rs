//! The output expectations of a compiled plan: `lineage` (CS-2.11), `explanation` (CS-2.12)
//! and `cache_key` (CS-2.14), and the comparison of `explain` with the plan (CS-2.16).

use std::path::Path;

use menura_core::CompiledPlanDocument;

use crate::canonical_json::JsonValue;
use crate::case::Expectation;
use crate::golden_json::{golden_mismatch, json_mismatch};

/// The `contract` of the document `explain` returns (CS-2.16).
const EXPLANATION_CONTRACT: &str = "menura.explanation";

/// The plan members the comparison of `explain` with the plan leaves out (CS-2.16).
const NOT_EXPLAINED: [&str; 3] = ["contract", "sql", "parameters"];

/// Why the plan does not meet the `lineage`, `explanation` or `cache_key` expectations of the
/// case, or `None`.
pub(crate) fn plan_outputs_mismatch(
    expect: &Expectation,
    plan: &CompiledPlanDocument,
    case_folder: &Path,
) -> Option<String> {
    lineage_mismatch(expect, plan, case_folder)
        .or_else(|| explanation_mismatch(expect, plan))
        .or_else(|| cache_key_mismatch(expect, plan))
}

/// The plan's `lineage` compared with its golden file (CS-2.11).
fn lineage_mismatch(
    expect: &Expectation,
    plan: &CompiledPlanDocument,
    case_folder: &Path,
) -> Option<String> {
    let file_name = expect.lineage.as_deref()?;
    match JsonValue::of(&plan.lineage) {
        Ok(lineage) => golden_mismatch(case_folder, file_name, &lineage),
        Err(reason) => Some(format!("the lineage cannot be read as JSON: {reason}")),
    }
}

/// The `code` of every explanation entry compared, in order, with the expected codes
/// (CS-2.12).
fn explanation_mismatch(expect: &Expectation, plan: &CompiledPlanDocument) -> Option<String> {
    let expected = expect.explanation.as_ref()?;
    let actual: Vec<Option<&str>> = plan
        .explanation
        .iter()
        .map(|entry| entry.code.as_deref())
        .collect();
    let matches = actual.len() == expected.len()
        && actual
            .iter()
            .zip(expected)
            .all(|(code, expected)| *code == Some(expected.as_str()));
    (!matches).then(|| {
        let actual: Vec<&str> = actual
            .iter()
            .map(|code| code.unwrap_or("(no code)"))
            .collect();
        format!(
            "the explanation codes are [{}], expected [{}]",
            actual.join(", "),
            expected.join(", ")
        )
    })
}

/// The plan's `cache_key.key` compared exactly (CS-2.14).
fn cache_key_mismatch(expect: &Expectation, plan: &CompiledPlanDocument) -> Option<String> {
    let expected = expect.cache_key.as_ref()?;
    let actual = &plan.cache_key.key;
    (actual != expected).then(|| {
        format!(
            "the cache key is {}, expected {}",
            actual.as_str(),
            expected.as_str()
        )
    })
}

/// Why the document `explain` returned differs from the compiled plan, or `None`: it must equal
/// the plan member for member, except `contract`, which is `menura.explanation`, and `sql` and
/// `parameters`, which are not compared (CS-2.16).
pub(crate) fn explain_mismatch(
    plan: &CompiledPlanDocument,
    explained: &JsonValue,
) -> Option<String> {
    let contract = explained.member("contract");
    if contract != Some(&JsonValue::String(EXPLANATION_CONTRACT.to_owned())) {
        return Some(format!(
            "the contract of `explain` is not `{EXPLANATION_CONTRACT}`"
        ));
    }
    let plan = match JsonValue::of(plan) {
        Ok(plan) => plan.without(&NOT_EXPLAINED),
        Err(reason) => return Some(format!("the plan cannot be read as JSON: {reason}")),
    };
    json_mismatch(&plan, &explained.without(&NOT_EXPLAINED), "compiled plan")
        .map(|reason| format!("`explain` differs from the compiled plan: {reason}"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use menura_contracts::Sha256Hash;
    use menura_contracts::types::common::{Decision, Step};

    use super::*;
    use crate::case_file::parse_case;

    const PLAN: &str = include_str!("../../../spec/contracts/examples/compiled-plan.example.json");

    fn plan() -> CompiledPlanDocument {
        serde_json::from_str(PLAN).unwrap()
    }

    fn expectation(members: &str) -> Expectation {
        let text = format!(
            "id: cs-unit-001\nrules: [CS-2.11]\nmodel: slice\nsecurity_context: auditor.json\n\
             query: {{contract: menura.query}}\nexpect:\n  outcome: plan\n  rows: e.csv\n{members}"
        );
        parse_case(&text).unwrap().expect
    }

    fn case_folder() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases/cs")
    }

    fn entry(code: Option<&str>) -> Decision {
        Decision {
            code: code.map(str::to_owned),
            step: Step::E5,
            subject: None,
            decision: "d".to_owned(),
            reason: "r".to_owned(),
        }
    }

    #[test]
    fn cs_2_11_the_lineage_matches_its_golden_file() {
        let mut plan = plan();
        plan.lineage.clear();
        let expect = expectation("  lineage: cs-unit-lineage-001.lineage.json\n");
        assert_eq!(plan_outputs_mismatch(&expect, &plan, &case_folder()), None);
    }

    #[test]
    fn cs_2_11_a_different_lineage_fails() {
        let expect = expectation("  lineage: cs-unit-lineage-001.lineage.json\n");
        let reason = plan_outputs_mismatch(&expect, &plan(), &case_folder()).unwrap();
        assert!(
            reason.contains("cs-unit-lineage-001.lineage.json"),
            "{reason}"
        );
    }

    #[test]
    fn cs_2_12_the_explanation_codes_match_in_order() {
        let mut plan = plan();
        plan.explanation = vec![entry(Some("PLN-7.1")), entry(Some("E5-2.2"))];
        let expect = expectation("  explanation: [PLN-7.1, E5-2.2]\n");
        assert_eq!(plan_outputs_mismatch(&expect, &plan, &case_folder()), None);
        let reversed = expectation("  explanation: [E5-2.2, PLN-7.1]\n");
        assert!(plan_outputs_mismatch(&reversed, &plan, &case_folder()).is_some());
        let shorter = expectation("  explanation: [PLN-7.1]\n");
        assert!(plan_outputs_mismatch(&shorter, &plan, &case_folder()).is_some());
    }

    #[test]
    fn cs_2_12_an_empty_list_asserts_an_empty_explanation() {
        let mut plan = plan();
        let expect = expectation("  explanation: []\n");
        plan.explanation.clear();
        assert_eq!(plan_outputs_mismatch(&expect, &plan, &case_folder()), None);
        plan.explanation = vec![entry(None)];
        let reason = plan_outputs_mismatch(&expect, &plan, &case_folder()).unwrap();
        assert!(reason.contains("(no code)"), "{reason}");
    }

    #[test]
    fn cs_2_12_an_entry_without_code_never_matches() {
        let mut plan = plan();
        plan.explanation = vec![entry(None)];
        let expect = expectation("  explanation: [\"(no code)\"]\n");
        assert!(plan_outputs_mismatch(&expect, &plan, &case_folder()).is_some());
    }

    #[test]
    fn cs_2_14_the_cache_key_is_compared_exactly() {
        let plan = plan();
        let key = plan.cache_key.key.as_str().to_owned();
        let same = expectation(&format!("  cache_key: \"{key}\"\n"));
        assert_eq!(plan_outputs_mismatch(&same, &plan, &case_folder()), None);
        let other = Sha256Hash::try_from("0".repeat(64)).unwrap();
        let different = expectation(&format!("  cache_key: \"{}\"\n", other.as_str()));
        let reason = plan_outputs_mismatch(&different, &plan, &case_folder()).unwrap();
        assert!(reason.contains("cache key"), "{reason}");
    }

    #[test]
    fn cs_2_without_output_expectations_nothing_is_compared() {
        assert_eq!(
            plan_outputs_mismatch(&expectation(""), &plan(), &case_folder()),
            None
        );
    }

    fn explained(plan: &CompiledPlanDocument) -> serde_json::Value {
        let mut value = serde_json::to_value(plan).unwrap();
        let members = value.as_object_mut().unwrap();
        members.insert("sql".to_owned(), "".into());
        members.insert(
            "parameters".to_owned(),
            serde_json::Value::Array(Vec::new()),
        );
        members.insert("contract".to_owned(), EXPLANATION_CONTRACT.into());
        value
    }

    #[test]
    fn cs_2_16_explain_equals_the_plan_but_contract_sql_and_parameters() {
        let plan = plan();
        let document = JsonValue::of(&explained(&plan)).unwrap();
        assert_eq!(explain_mismatch(&plan, &document), None);
    }

    #[test]
    fn cs_2_16_explain_with_another_contract_fails() {
        let plan = plan();
        let mut value = explained(&plan);
        value["contract"] = "menura.compiled_plan".into();
        let reason = explain_mismatch(&plan, &JsonValue::of(&value).unwrap()).unwrap();
        assert!(reason.contains(EXPLANATION_CONTRACT), "{reason}");
    }

    #[test]
    fn cs_2_16_explain_with_another_member_fails() {
        let plan = plan();
        let mut other = explained(&plan);
        other["data_source"] = "elsewhere".into();
        assert!(explain_mismatch(&plan, &JsonValue::of(&other).unwrap()).is_some());
    }
}
