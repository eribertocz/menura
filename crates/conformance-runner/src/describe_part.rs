//! The document part of an `outcome: describe` case (CS-2.15): `describe` runs with the case's
//! security context and its output document is compared with the golden file (CS-2.10).

use std::path::Path;

use menura_core::{DescribeResult, Handle, describe};

use crate::canonical_json::JsonValue;
use crate::case::Case;
use crate::golden_json::golden_mismatch;
use crate::report::CaseStatus;

/// The status of the document part of `case`, whose model loaded into `handle`: `describe`
/// runs with `security_context`, and the document it returns, the model description or the
/// compile error of a context that is not admitted (DSC-3.12), is compared with the golden
/// file `document` next to the case.
pub(crate) fn check_describe(
    case: &Case,
    handle: &Handle,
    security_context: &[u8],
    case_folder: &Path,
) -> CaseStatus {
    let Some(file_name) = case.expect.document.as_deref() else {
        return CaseStatus::Failed("`outcome: describe` requires `document` (CS-2.15)".to_owned());
    };
    let produced = match describe(handle, security_context) {
        DescribeResult::Description(document) => JsonValue::of(&document),
        DescribeResult::Error(error) => JsonValue::of(&error),
    };
    let mismatch = match produced {
        Ok(produced) => golden_mismatch(case_folder, file_name, &produced),
        Err(reason) => Some(format!("the document cannot be read as JSON: {reason}")),
    };
    mismatch.map_or(CaseStatus::Passed, CaseStatus::Failed)
}
