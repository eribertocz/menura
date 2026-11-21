//! The load part of a case (CS-2.8): the model of the case is loaded through `core`, and the
//! result is compared with the expectation. A `load_error` case is checked fully; for the other
//! outcomes, the model must load, and the loaded model goes on to the query part, which also
//! checks the expected warnings. The expected location of the first error is compared as CS-2.9
//! says.

use std::path::Path;

use menura_core::{CompileErrorDocument, Handle, LoadOptions, load};

use crate::case::{Case, Expectation, Outcome};
use crate::diagnostic_check::{Part, describe, location_mismatch, missing_warnings};
use crate::error::RunnerError;
use crate::fixture::read_bytes;
use crate::report::CaseStatus;

/// The result of the load part of a case.
#[derive(Debug, Clone)]
pub(crate) enum LoadOutcome {
    /// The case is decided by its load part: a `load_error` case, or a model that does not load.
    Checked(CaseStatus),
    /// The model loads, and the query part of the case decides it.
    Loaded(Box<Handle>),
}

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
) -> LoadOutcome {
    let loaded = load(
        &documents.model,
        &documents.catalog_schema,
        documents.catalog_state.as_deref(),
        options,
    );
    let expect = &case.expect;
    let status = match (loaded, expect.outcome) {
        (Err(error), Outcome::LoadError) => check_load_error(expect, &error),
        (Err(error), _) => CaseStatus::Failed(format!(
            "the model does not load: {}",
            describe(error.errors.first())
        )),
        (Ok(_), Outcome::LoadError) => CaseStatus::Failed(format!(
            "the model loads, but the case expects the load error {}",
            expect.error.as_deref().unwrap_or("none")
        )),
        (Ok(handle), _) => return LoadOutcome::Loaded(Box::new(handle)),
    };
    LoadOutcome::Checked(status)
}

/// Compares a rejected load with the expectation of a `load_error` case: the first error code,
/// its location when the case gives one (CS-2.9), and the expected warnings.
fn check_load_error(expect: &Expectation, error: &CompileErrorDocument) -> CaseStatus {
    let first = error.errors.first();
    if first.map(|diagnostic| diagnostic.code.as_str()) != expect.error.as_deref() {
        return CaseStatus::Failed(format!(
            "the first load error is {}, expected {}",
            describe(first),
            expect.error.as_deref().unwrap_or("none")
        ));
    }
    let reported_location = first.and_then(|diagnostic| diagnostic.location.as_ref());
    if let Some(reason) = expect
        .location
        .as_ref()
        .and_then(|expected| location_mismatch(expected, reported_location, Part::Load))
    {
        return CaseStatus::Failed(reason);
    }
    match missing_warnings(&expect.warnings, error.warnings.iter(), Part::Load) {
        Some(reason) => CaseStatus::Failed(reason),
        None => CaseStatus::Passed,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::case_file::parse_case;
    use crate::load_options::resolve_load_options;

    fn outcome(case_text: &str, model: &str) -> LoadOutcome {
        let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../conformance/models")
            .join(model);
        let case = parse_case(case_text).unwrap();
        let documents = ModelDocuments::read(&folder).unwrap();
        check_load(&case, &documents, &resolve_load_options(None))
    }

    fn status(case_text: &str, model: &str) -> CaseStatus {
        match outcome(case_text, model) {
            LoadOutcome::Checked(status) => status,
            LoadOutcome::Loaded(_) => panic!("the load part decides this case"),
        }
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
    fn cs_2_8_a_case_whose_model_loads_goes_on_to_its_query_part() {
        let case = "
id: sample-plan-001
rules: [CS-2]
model: sample
security_context: analyst.json
query: {}
expect: {outcome: plan, rows: sample-plan-001.csv}
";
        assert!(matches!(outcome(case, "sample"), LoadOutcome::Loaded(_)));
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

    const LOCATED_MISSING_SOURCE: &str = "
id: sample-load-error-001
rules: [CS-2.9]
model: sample-missing-source
expect:
  outcome: load_error
  error: E0D-02
  location: {document: model, pointer: /semantic_model/0/datasets/0/source, line: 10, column: 17}
";

    #[test]
    fn cs_2_9_a_load_error_case_passes_when_its_location_matches() {
        assert_eq!(
            status(LOCATED_MISSING_SOURCE, "sample-missing-source"),
            CaseStatus::Passed
        );
    }

    #[test]
    fn cs_2_9_a_load_error_case_passes_without_line_and_column() {
        let case = LOCATED_MISSING_SOURCE.replace(", line: 10, column: 17", "");
        assert_eq!(status(&case, "sample-missing-source"), CaseStatus::Passed);
    }

    fn assert_location_fails(case: &str) {
        let CaseStatus::Failed(reason) = status(case, "sample-missing-source") else {
            panic!("the case fails");
        };
        assert!(reason.contains("location"), "{reason}");
    }

    #[test]
    fn cs_2_9_a_different_document_fails() {
        assert_location_fails(
            &LOCATED_MISSING_SOURCE.replace("document: model", "document: catalog_schema"),
        );
    }

    #[test]
    fn cs_2_9_a_different_pointer_fails() {
        assert_location_fails(&LOCATED_MISSING_SOURCE.replace("datasets/0/source", "datasets/0"));
    }

    #[test]
    fn cs_2_9_a_different_line_fails() {
        assert_location_fails(&LOCATED_MISSING_SOURCE.replace("line: 10", "line: 11"));
    }

    #[test]
    fn cs_2_9_a_different_column_fails() {
        assert_location_fails(&LOCATED_MISSING_SOURCE.replace("column: 17", "column: 1"));
    }
}
