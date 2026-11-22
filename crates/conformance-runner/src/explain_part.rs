//! The explain part of a case (CS-2.16): from 0.7.0, the runner also runs `explain` with the
//! inputs of every `outcome: plan` case and compares the returned document with the compiled
//! plan; from 0.8.0, it runs `explain` for every `outcome: error` case too, which must return the
//! identical compile error.
//!
//! `explain` is called with the case's compile options and `audit: true`
//! ([`crate::compile_options::explain_options`]). The `explain` of `core` returns a
//! [`CompileResult`] and no audit record, so the check of PLN-8.3 holds by its signature.

use menura_contracts::ContractVersion;
use menura_core::{CompileErrorDocument, CompileResult, CompiledPlanDocument};

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

/// The version from which every `outcome: error` case also runs `explain` (CS-2.16).
const EXPLAIN_ERRORS_FROM: ContractVersion = ContractVersion {
    major: 0,
    minor: 8,
    patch: 0,
};

/// Whether `case` also runs `explain` in this build: an `outcome: plan` case from 0.7.0, an
/// `outcome: error` case from 0.8.0 (CS-2.16).
pub(crate) fn runs_explain(case: &Case) -> bool {
    outcome_runs_explain(case.expect.outcome, BUILD_VERSION)
}

/// Whether the build of `version` runs `explain` for every case of `outcome` (CS-2.16).
fn outcome_runs_explain(outcome: Outcome, version: &str) -> bool {
    let from = match outcome {
        Outcome::Plan => EXPLAIN_FROM,
        Outcome::Error => EXPLAIN_ERRORS_FROM,
        _ => return false,
    };
    ContractVersion::parse(version).is_some_and(|version| version >= from)
}

/// Why `explained`, what `explain` returned for the inputs on which `compile` failed with
/// `error`, is not the identical compile error, or `None` (CS-2.16, PLN-8.2).
pub(crate) fn explained_error_mismatch(
    error: &CompileErrorDocument,
    explained: &CompileResult,
) -> Option<String> {
    match explained {
        CompileResult::Error(explained) if explained == error => None,
        CompileResult::Error(explained) => Some(format!(
            "`explain` returns another compile error than `compile`: {}",
            describe(explained.errors.first())
        )),
        CompileResult::Plan(_) => {
            Some("`explain` returns a plan where the query does not compile".to_owned())
        }
    }
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

    fn compile_error(code: &str) -> CompileErrorDocument {
        CompileErrorDocument {
            contract: CompileErrorContract::CompileError,
            contract_version: plan().contract_version,
            errors: vec![Diagnostic {
                code: code.to_owned(),
                message: "m".to_owned(),
                location: None,
                alternatives: None,
            }],
            warnings: Vec::new(),
        }
    }

    #[test]
    fn cs_2_16_explain_runs_for_plan_cases_from_0_7_0() {
        assert!(!outcome_runs_explain(Outcome::Plan, "0.6.0"));
        assert!(outcome_runs_explain(Outcome::Plan, "0.7.0"));
        assert!(outcome_runs_explain(Outcome::Plan, "1.0.0"));
    }

    #[test]
    fn cs_2_16_explain_runs_for_error_cases_from_0_8_0() {
        assert!(!outcome_runs_explain(Outcome::Error, "0.7.0"));
        assert!(outcome_runs_explain(Outcome::Error, "0.8.0"));
        assert!(outcome_runs_explain(Outcome::Error, "1.0.0"));
    }

    #[test]
    fn cs_2_16_explain_runs_only_for_plan_and_error_cases() {
        let plan_runs = outcome_runs_explain(Outcome::Plan, BUILD_VERSION);
        let error_runs = outcome_runs_explain(Outcome::Error, BUILD_VERSION);
        assert_eq!(runs_explain(&case("plan", "  rows: e.csv\n")), plan_runs);
        assert_eq!(runs_explain(&case("error", "  error: E2-01\n")), error_runs);
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
        let error = compile_error("E_INTERNAL");
        let reason = explained_mismatch(&plan(), &CompileResult::Error(error)).unwrap();
        assert!(reason.contains("E_INTERNAL"), "{reason}");
    }

    #[test]
    fn pln_8_2_the_identical_compile_error_passes() {
        let error = compile_error("E_NAME_NOT_FOUND");
        let explained = CompileResult::Error(error.clone());
        assert_eq!(explained_error_mismatch(&error, &explained), None);
    }

    #[test]
    fn pln_8_2_another_compile_error_fails() {
        let error = compile_error("E_NAME_NOT_FOUND");
        let explained = CompileResult::Error(compile_error("E_AMBIGUOUS_PATH"));
        let reason = explained_error_mismatch(&error, &explained).unwrap();
        assert!(reason.contains("E_AMBIGUOUS_PATH"), "{reason}");
    }

    #[test]
    fn pln_8_2_a_plan_where_compile_fails_fails() {
        let error = compile_error("E_NAME_NOT_FOUND");
        let explained = CompileResult::Plan(Box::new(explained(&plan())));
        assert!(explained_error_mismatch(&error, &explained).is_some());
    }
}
