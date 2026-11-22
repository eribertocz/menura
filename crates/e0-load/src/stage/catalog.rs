//! Admission of the catalog schema before E0.a (E0-3.2, CAT-9.1).

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_contracts::{admit_catalog_schema, schema_former_name_warning};

use super::report::Report;

/// The code of a catalog schema that is not valid UTF-8 JSON (CAT-9.1).
const NOT_JSON: &str = "ECAT-01";

/// Admits the catalog schema `bytes`, or adds its error to `report` and returns `None`. An
/// admitted schema with the former contract name adds its `WCAT-05` warning (CON-0.5).
///
/// Bytes that are not UTF-8 are not JSON: `ECAT-01` at the empty pointer (CAT-9.1).
pub(super) fn admit(bytes: &[u8], report: &mut Report) -> Option<CatalogSchemaDocument> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        report.add_error(not_utf8());
        return None;
    };
    match admit_catalog_schema(text) {
        Ok(schema) => {
            if let Some(warning) = schema_former_name_warning(&schema) {
                report.add_catalog_warning(&warning);
            }
            Some(schema)
        }
        Err(error) => {
            report.add_catalog_error(&error);
            None
        }
    }
}

fn not_utf8() -> Diagnostic {
    Diagnostic {
        code: NOT_JSON.to_owned(),
        message: "the catalog schema is not valid UTF-8, so it is not JSON; publish it as UTF-8 \
                  JSON text"
            .to_owned(),
        location: Some(DiagnosticLocation {
            document: InputDocument::CatalogSchema,
            pointer: String::new(),
            line: None,
            column: None,
        }),
        alternatives: None,
    }
}
