//! The document part of an `outcome: integrity_checks` case: `integrity_checks` with the
//! dialect `duckdb`, compared with the case's golden `document` (CS-2.15, CS-2.10).

use std::path::Path;

use menura_core::{Dialect, Handle, integrity_checks};

use crate::canonical_json::JsonValue;
use crate::case::Case;
use crate::diagnostic_check::describe;
use crate::golden_json::golden_mismatch;
use crate::report::CaseStatus;

/// Runs `integrity_checks` over `handle` and compares the document with the golden file the
/// case names in `document` (CS-2.15). It needs no security context.
pub(crate) fn integrity_checks_part(
    case: &Case,
    handle: &Handle,
    case_folder: &Path,
) -> CaseStatus {
    let Some(file_name) = case.expect.document.as_deref() else {
        return CaseStatus::Failed("`outcome: integrity_checks` requires `document`".to_owned());
    };
    let document = match integrity_checks(handle, Dialect::Duckdb) {
        Ok(document) => document,
        Err(error) => {
            return CaseStatus::Failed(format!(
                "`integrity_checks` returned an error: {}",
                describe(error.errors.first())
            ));
        }
    };
    let produced = match JsonValue::of(&document) {
        Ok(produced) => produced,
        Err(reason) => {
            return CaseStatus::Failed(format!("the document cannot be read as JSON: {reason}"));
        }
    };
    golden_mismatch(case_folder, file_name, &produced)
        .map_or(CaseStatus::Passed, CaseStatus::Failed)
}
