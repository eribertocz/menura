//! The load part of a case (CS-2.8): the model of the case is loaded through `core`, and the
//! result is compared with the expectation. A `load_error` case is checked fully; for the other
//! outcomes, the model must load with the expected warnings, and the query part stays pending.

use std::path::Path;

use semantic_compiler_core::{Diagnostic, LoadOptions, load};

use crate::case::{Case, Outcome};
use crate::error::RunnerError;
use crate::fixture::read_bytes;
use crate::report::CaseStatus;

/// Why the query part of a case is pending: stages E1 to E6 come in 0.3.0 (CS-2.8).
pub const QUERY_PENDING_REASON: &str =
    "the load part passed; the query part needs stages E1 to E6, implemented from 0.3.0";

/// The documents `load` reads for the model of a case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelDocuments {
    model: Vec<u8>,
    catalog_schema: Vec<u8>,
    catalog_state: Option<Vec<u8>>,
}

impl ModelDocuments {
    /// Reads `model.yaml`, `catalog-schema.json` and, when present, `catalog-state.json` of the
    /// model folder `folder` (CS-1), as bytes.
    pub(crate) fn read(folder: &Path) -> Result<Self, RunnerError> {
        let state_path = folder.join("catalog-state.json");
        let catalog_state = if state_path.is_file() {
            Some(read_bytes(&state_path)?)
        } else {
            None
        };
        Ok(Self {
            model: read_bytes(&folder.join("model.yaml"))?,
            catalog_schema: read_bytes(&folder.join("catalog-schema.json"))?,
            catalog_state,
        })
    }
}

/// Loads the model of `case` with `options` and compares the result with its expectation.
pub(crate) fn check_load(
    case: &Case,
    documents: &ModelDocuments,
    options: &LoadOptions,
) -> CaseStatus {
    let loaded = load(
        &documents.model,
        &documents.catalog_schema,
        documents.catalog_state.as_deref(),
        options,
    );
    let expect = &case.expect;
    match (loaded, expect.outcome) {
        (Err(error), Outcome::LoadError) => {
            let first = error
                .errors
                .first()
                .map(|diagnostic| diagnostic.code.as_str());
            if first != expect.error.as_deref() {
                return CaseStatus::Failed(format!(
                    "the first load error is {}, expected {}",
                    describe(error.errors.first()),
                    expect.error.as_deref().unwrap_or("none")
                ));
            }
            match missing_warnings(&expect.warnings, error.warnings.iter()) {
                Some(reason) => CaseStatus::Failed(reason),
                None => CaseStatus::Passed,
            }
        }
        (Err(error), _) => CaseStatus::Failed(format!(
            "the model does not load: {}",
            describe(error.errors.first())
        )),
        (Ok(_), Outcome::LoadError) => CaseStatus::Failed(format!(
            "the model loads, but the case expects the load error {}",
            expect.error.as_deref().unwrap_or("none")
        )),
        (Ok(handle), _) => match missing_warnings(&expect.warnings, handle.warnings()) {
            Some(reason) => CaseStatus::Failed(reason),
            None => CaseStatus::Pending(QUERY_PENDING_REASON.to_owned()),
        },
    }
}

/// Why the reported warnings lack an expected code, or `None` when every expected code is
/// reported, each by its own warning.
fn missing_warnings<'report>(
    expected: &[String],
    reported: impl Iterator<Item = &'report Diagnostic>,
) -> Option<String> {
    let mut remaining: Vec<&str> = reported.map(|warning| warning.code.as_str()).collect();
    let reported = remaining.join(", ");
    for code in expected {
        let Some(index) = remaining.iter().position(|reported| reported == code) else {
            return Some(format!(
                "the load warning {code} is not reported; the warnings are [{reported}]"
            ));
        };
        remaining.remove(index);
    }
    None
}

/// A diagnostic as `CODE (message)`, for failure reasons.
fn describe(diagnostic: Option<&Diagnostic>) -> String {
    diagnostic.map_or_else(
        || "none".to_owned(),
        |diagnostic| format!("{} ({})", diagnostic.code, diagnostic.message),
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::case_file::parse_case;
    use crate::load_options::resolve_load_options;

    fn status(case_text: &str, model: &str) -> CaseStatus {
        let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/models")
            .join(model);
        let case = parse_case(case_text).unwrap();
        let documents = ModelDocuments::read(&folder).unwrap();
        check_load(&case, &documents, &resolve_load_options(None))
    }

    const MISSING_SOURCE: &str = "
id: sample-load-error-001
rules: [CS-2]
model: sample-missing-source
expect: {outcome: load_error, error: E0D-02}
";

    #[test]
    fn cs_2_8_a_load_error_case_passes_when_its_first_error_matches() {
        assert_eq!(
            status(MISSING_SOURCE, "sample-missing-source"),
            CaseStatus::Passed
        );
    }

    #[test]
    fn cs_2_8_a_load_error_case_fails_with_another_first_error() {
        let case = MISSING_SOURCE.replace("E0D-02", "E0D-01");
        assert!(matches!(
            status(&case, "sample-missing-source"),
            CaseStatus::Failed(_)
        ));
    }

    #[test]
    fn cs_2_8_a_load_error_case_fails_when_the_model_loads() {
        assert!(matches!(
            status(MISSING_SOURCE, "sample"),
            CaseStatus::Failed(_)
        ));
    }

    #[test]
    fn cs_2_8_an_expected_warning_must_be_reported() {
        let case = MISSING_SOURCE.replace("error: E0D-02}", "error: E0D-02, warnings: [W0D-01]}");
        let CaseStatus::Failed(reason) = status(&case, "sample-missing-source") else {
            panic!("the case fails");
        };
        assert!(reason.contains("W0D-01"), "{reason}");
    }

    #[test]
    fn cs_2_8_the_query_part_of_a_case_whose_model_loads_is_pending() {
        let case = "
id: sample-plan-001
rules: [CS-2]
model: sample
security_context: analyst.json
query: {}
expect: {outcome: plan, rows: sample-plan-001.csv}
";
        assert_eq!(
            status(case, "sample"),
            CaseStatus::Pending(QUERY_PENDING_REASON.to_owned())
        );
    }

    #[test]
    fn cs_2_8_a_case_whose_model_does_not_load_fails_its_load_part() {
        let case = "
id: sample-plan-001
rules: [CS-2]
model: sample-missing-source
security_context: analyst.json
query: {}
expect: {outcome: plan, rows: sample-plan-001.csv}
";
        assert!(matches!(
            status(case, "sample-missing-source"),
            CaseStatus::Failed(_)
        ));
    }

    #[test]
    fn cs_2_8_each_expected_warning_needs_its_own_warning() {
        let warning = |code: &str| Diagnostic {
            code: code.to_owned(),
            message: String::new(),
            location: None,
        };
        let reported = [warning("W0F-01")];
        let expected = ["W0F-01".to_owned(), "W0F-01".to_owned()];
        assert!(missing_warnings(&expected, reported.iter()).is_some());
        assert!(missing_warnings(&expected[..1], reported.iter()).is_none());
    }
}
