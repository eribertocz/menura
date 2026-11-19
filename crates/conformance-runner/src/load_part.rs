//! The load part of a case (CS-2.8): the model of the case is loaded through `core`, and the
//! result is compared with the expectation. A `load_error` case is checked fully; for the other
//! outcomes, the model must load with the expected warnings, and the query part stays pending.
//! The expected location of the first error is compared as CS-2.9 says.

use std::path::Path;

use semantic_compiler_core::{
    CompileErrorDocument, Diagnostic, DiagnosticLocation, InputDocument, LoadOptions, load,
};

use crate::case::{Case, Expectation, ExpectedLocation, Outcome};
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
        (Err(error), Outcome::LoadError) => check_load_error(expect, &error),
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
        .and_then(|expected| location_mismatch(expected, reported_location))
    {
        return CaseStatus::Failed(reason);
    }
    match missing_warnings(&expect.warnings, error.warnings.iter()) {
        Some(reason) => CaseStatus::Failed(reason),
        None => CaseStatus::Passed,
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

/// Why the location of the first error differs from `expected`, or `None` when it matches:
/// `document` and `pointer` exactly, and `line` and `column` when the case gives them (CS-2.9).
fn location_mismatch(
    expected: &ExpectedLocation,
    reported: Option<&DiagnosticLocation>,
) -> Option<String> {
    let Some(reported) = reported else {
        return Some("the first load error has no location, but the case expects one".to_owned());
    };
    let matches = expected.document == reported.document
        && expected.pointer == reported.pointer
        && expected.line.is_none_or(|line| reported.line == Some(line))
        && expected
            .column
            .is_none_or(|column| reported.column == Some(column));
    if matches {
        None
    } else {
        Some(format!(
            "the location of the first load error is {}, expected {}",
            describe_location(
                reported.document,
                &reported.pointer,
                reported.line,
                reported.column
            ),
            describe_location(
                expected.document,
                &expected.pointer,
                expected.line,
                expected.column
            )
        ))
    }
}

/// A location as `document pointer line:column`, for failure reasons.
fn describe_location(
    document: InputDocument,
    pointer: &str,
    line: Option<u64>,
    column: Option<u64>,
) -> String {
    let position =
        |value: Option<u64>| value.map_or_else(|| "-".to_owned(), |value| value.to_string());
    format!(
        "{document:?} `{pointer}` {}:{}",
        position(line),
        position(column)
    )
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

    fn expected_location(line: Option<u64>, column: Option<u64>) -> ExpectedLocation {
        ExpectedLocation {
            document: InputDocument::Model,
            pointer: "/a".to_owned(),
            line,
            column,
        }
    }

    fn diagnostic_location(line: Option<u64>, column: Option<u64>) -> DiagnosticLocation {
        DiagnosticLocation {
            document: InputDocument::Model,
            pointer: "/a".to_owned(),
            line,
            column,
        }
    }

    #[test]
    fn cs_2_9_a_first_error_without_location_fails() {
        assert!(location_mismatch(&expected_location(None, None), None).is_some());
    }

    #[test]
    fn cs_2_9_an_expected_line_needs_a_reported_line() {
        let reported = diagnostic_location(None, None);
        assert!(location_mismatch(&expected_location(Some(1), None), Some(&reported)).is_some());
        assert!(location_mismatch(&expected_location(None, Some(1)), Some(&reported)).is_some());
    }

    #[test]
    fn cs_2_9_line_and_column_are_compared_only_when_the_case_gives_them() {
        let reported = diagnostic_location(Some(3), Some(4));
        assert!(location_mismatch(&expected_location(None, None), Some(&reported)).is_none());
        assert!(location_mismatch(&expected_location(Some(3), Some(4)), Some(&reported)).is_none());
    }
}
