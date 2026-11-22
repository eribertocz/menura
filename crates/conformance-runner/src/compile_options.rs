//! The compile options a case is compiled and explained with (CS-2.13, CS-2.16, CS-2.18).

use menura_core::CompileOptions;

use crate::case::Case;

/// The options `compile` runs with for `case`: its `actor` (CS-2.18), and `audit: true` only
/// when it expects an audit record (CS-2.13). Every other option keeps its default.
///
/// # Errors
///
/// A reason when the case's `actor` is not a JSON string, the only value the compile option of
/// the build under test accepts (PLN-6.8).
pub(crate) fn compile_options(case: &Case) -> Result<CompileOptions, String> {
    Ok(CompileOptions {
        audit: case.expect.audit.is_some(),
        actor: actor(case)?,
        ..CompileOptions::default()
    })
}

/// The options `explain` runs with: those of `compile`, with `audit: true` (CS-2.16).
pub(crate) fn explain_options(compile: &CompileOptions) -> CompileOptions {
    CompileOptions {
        audit: true,
        ..compile.clone()
    }
}

/// The case's `actor`, or `None` when it gives none (CS-2.18).
fn actor(case: &Case) -> Result<Option<String>, String> {
    let value = case
        .compile_options
        .as_ref()
        .and_then(|options| options.actor.as_ref());
    match value {
        None => Ok(None),
        Some(serde_json::Value::String(actor)) => Ok(Some(actor.clone())),
        Some(other) => Err(format!(
            "the compile option `actor` of this build accepts only a string, not {other}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case_file::parse_case;

    fn case(top: &str, expect: &str) -> Case {
        let text = format!(
            "id: cs-unit-001\nrules: [CS-2.18]\nmodel: slice\nsecurity_context: auditor.json\n\
             query: {{contract: menura.query}}\n{top}expect: {{outcome: error, error: E2-01{expect}}}\n"
        );
        parse_case(&text).unwrap()
    }

    #[test]
    fn cs_2_18_every_option_keeps_its_default_without_compile_options() {
        assert_eq!(
            compile_options(&case("", "")),
            Ok(CompileOptions::default())
        );
    }

    #[test]
    fn cs_2_18_the_actor_is_passed_to_compile() {
        let options = compile_options(&case("compile_options: {actor: agent-7}\n", "")).unwrap();
        assert_eq!(options.actor.as_deref(), Some("agent-7"));
        assert!(!options.audit);
    }

    #[test]
    fn cs_2_13_an_audit_expectation_compiles_with_audit() {
        let options = compile_options(&case("", ", audit: a.json")).unwrap();
        assert!(options.audit);
    }

    #[test]
    fn cs_2_18_an_actor_that_is_not_a_string_cannot_be_passed() {
        let case = case("compile_options: {actor: {id: agent-7}}\n", "");
        let reason = compile_options(&case).unwrap_err();
        assert!(reason.contains("actor"), "{reason}");
    }

    #[test]
    fn cs_2_16_explain_runs_with_audit_and_the_case_options() {
        let compile = compile_options(&case("compile_options: {actor: agent-7}\n", "")).unwrap();
        let explain = explain_options(&compile);
        assert!(explain.audit);
        assert_eq!(explain.actor.as_deref(), Some("agent-7"));
    }
}
