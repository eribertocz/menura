//! The compile options a case is compiled and explained with (CS-2.13, CS-2.16, CS-2.18).

use menura_core::CompileOptions;

use crate::case::Case;

/// The options `compile` runs with for `case`: its `actor` (CS-2.18), and `audit: true` only
/// when it expects an audit record (CS-2.13). Every other option keeps its default.
pub(crate) fn compile_options(case: &Case) -> CompileOptions {
    CompileOptions {
        audit: case.expect.audit.is_some(),
        actor: case
            .compile_options
            .as_ref()
            .and_then(|options| options.actor.clone()),
        ..CompileOptions::default()
    }
}

/// The options `explain` runs with: those of `compile`, with `audit: true` (CS-2.16).
pub(crate) fn explain_options(compile: &CompileOptions) -> CompileOptions {
    CompileOptions {
        audit: true,
        ..compile.clone()
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
        assert_eq!(compile_options(&case("", "")), CompileOptions::default());
    }

    #[test]
    fn cs_2_18_the_actor_is_passed_to_compile() {
        let options = compile_options(&case("compile_options: {actor: agent-7}\n", ""));
        assert_eq!(options.actor.as_deref(), Some("agent-7"));
        assert!(!options.audit);
    }

    #[test]
    fn cs_2_13_an_audit_expectation_compiles_with_audit() {
        assert!(compile_options(&case("", ", audit: a.json")).audit);
    }

    #[test]
    fn cs_2_16_explain_runs_with_audit_and_the_case_options() {
        let compile = compile_options(&case("compile_options: {actor: agent-7}\n", ""));
        let explain = explain_options(&compile);
        assert!(explain.audit);
        assert_eq!(explain.actor.as_deref(), Some("agent-7"));
    }
}
