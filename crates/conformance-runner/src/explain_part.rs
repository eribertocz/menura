//! The explain part of a case (CS-2.16): from 0.7.0, the runner also runs `explain` with the
//! inputs of every `outcome: plan` case and compares the returned document with the compiled
//! plan.

use menura_contracts::ContractVersion;
use menura_core::{CompileResult, CompiledPlanDocument};

use crate::canonical_json::JsonValue;
use crate::case::{Case, Outcome};
use crate::diagnostic_check::describe;
use crate::plan_outputs::explain_mismatch;

/// The version of the build under test: the workspace version.
const BUILD_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The version from which every `outcome: plan` case also runs `explain` (CS-2.16).
const EXPLAIN_FROM: ContractVersion = ContractVersion {
    major: 0,
    minor: 7,
    patch: 0,
};

/// Whether `case` also runs `explain` in this build: an `outcome: plan` case, from 0.7.0
/// (CS-2.16).
pub(crate) fn runs_explain(case: &Case) -> bool {
    case.expect.outcome == Outcome::Plan && version_runs_explain(BUILD_VERSION)
}

/// Whether the build of `version` runs `explain` for every `outcome: plan` case (CS-2.16).
fn version_runs_explain(version: &str) -> bool {
    ContractVersion::parse(version).is_some_and(|version| version >= EXPLAIN_FROM)
}

/// Why `explained`, what `explain` returned for the inputs that compiled to `plan`, differs from
/// the plan, or `None` (CS-2.16).
pub(crate) fn explained_mismatch(
    plan: &CompiledPlanDocument,
    explained: &CompileResult,
) -> Option<String> {
    let document = match explained {
        CompileResult::Plan(document) => document,
        CompileResult::Error(error) => {
            return Some(format!(
                "`explain` fails where the query compiles: {}",
                describe(error.errors.first())
            ));
        }
    };
    match JsonValue::of(document) {
        Ok(document) => explain_mismatch(plan, &document),
        Err(reason) => Some(format!("`explain` cannot be read as JSON: {reason}")),
    }
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::compiled_plan::{
        CompileErrorContract, CompileErrorDocument, CompiledPlanContract, Diagnostic,
    };

    use super::*;
    use crate::case_file::parse_case;

    const PLAN: &str = include_str!("../../../spec/contracts/examples/compiled-plan.example.json");

    fn plan() -> CompiledPlanDocument {
        serde_json::from_str(PLAN).unwrap()
    }

    fn case(outcome: &str, members: &str) -> Case {
        let text = format!(
            "id: cs-unit-001\nrules: [CS-2.16]\nmodel: slice\nsecurity_context: auditor.json\n\
             query: {{contract: menura.query}}\nexpect:\n  outcome: {outcome}\n{members}"
        );
        parse_case(&text).unwrap()
    }

    fn explained(plan: &CompiledPlanDocument) -> CompiledPlanDocument {
        CompiledPlanDocument {
            contract: CompiledPlanContract::Explanation,
            sql: String::new(),
            parameters: Vec::new(),
            ..plan.clone()
        }
    }

    #[test]
    fn cs_2_16_explain_runs_from_0_7_0() {
        assert!(!version_runs_explain("0.6.0"));
        assert!(version_runs_explain("0.7.0"));
        assert!(version_runs_explain("1.0.0"));
    }

    #[test]
    fn cs_2_16_explain_runs_only_for_plan_cases() {
        let runs = version_runs_explain(BUILD_VERSION);
        assert_eq!(runs_explain(&case("plan", "  rows: e.csv\n")), runs);
        assert!(!runs_explain(&case("error", "  error: E2-01\n")));
        assert!(!runs_explain(&case(
            "runtime_error",
            "  error: RADD-01\n  rows: e.csv\n"
        )));
    }

    #[test]
    fn cs_2_16_the_explanation_of_the_plan_passes() {
        let plan = plan();
        let result = CompileResult::Plan(Box::new(explained(&plan)));
        assert_eq!(explained_mismatch(&plan, &result), None);
    }

    #[test]
    fn cs_2_16_an_explanation_with_another_member_fails() {
        let plan = plan();
        let mut document = explained(&plan);
        document.data_source = "elsewhere".to_owned();
        let result = CompileResult::Plan(Box::new(document));
        assert!(explained_mismatch(&plan, &result).is_some());
    }

    #[test]
    fn cs_2_16_an_explain_error_fails() {
        let error = CompileErrorDocument {
            contract: CompileErrorContract::CompileError,
            contract_version: plan().contract_version,
            errors: vec![Diagnostic {
                code: "E_INTERNAL".to_owned(),
                message: "m".to_owned(),
                location: None,
                alternatives: None,
            }],
            warnings: Vec::new(),
        };
        let reason = explained_mismatch(&plan(), &CompileResult::Error(error)).unwrap();
        assert!(reason.contains("E_INTERNAL"), "{reason}");
    }
}
