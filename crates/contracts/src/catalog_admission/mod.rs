//! Admission of the catalog schema and catalog state documents, in the order of checks of
//! CAT-9.1: JSON, then `contract` and `contract_version`, then the JSON Schema, then the CAT-8
//! rules, then, for the state document, `schema_document_id`.

mod period;
mod schema_rules;
mod state_rules;

use std::collections::HashSet;
use std::hash::Hash;

use crate::admission::{AdmissionCode, AdmissionError, IncomingContract, decode};
use crate::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};

/// Admits a catalog schema document (CAT-3, CAT-8, CAT-9).
///
/// # Errors
///
/// The first failure, in the order of CAT-9.1: `ECAT-01` when the text is not JSON, `ECAT-02`
/// when `contract` or `contract_version` is not supported, `ECAT-01` when the document does not
/// satisfy its JSON Schema, and `ECAT-03` when it violates a CAT-8 rule. Each error makes `load`
/// fail.
///
/// # Example
///
/// ```
/// use semantic_compiler_contracts::{AdmissionCode, admit_catalog_schema};
///
/// let error = admit_catalog_schema("{}").unwrap_err();
/// assert_eq!(error.code, AdmissionCode::Ecat02);
/// ```
pub fn admit_catalog_schema(text: &str) -> Result<CatalogSchemaDocument, AdmissionError> {
    let contract = IncomingContract::CatalogSchema;
    let document = contract.parse_json(text)?;
    contract.check_contract(&document)?;
    contract.validate(&document)?;
    let schema: CatalogSchemaDocument = decode(contract, &document)?;
    schema_rules::check(&schema)
        .map_err(|violation| violation.into_error(contract, AdmissionCode::Ecat03))?;
    Ok(schema)
}

/// Admits a catalog state document against `schema`, the admitted schema document in use
/// (CAT-2.2, CAT-3, CAT-8, CAT-9).
///
/// The CAT-8 rules that need a schema (CAT-8.4, CAT-8.5) are checked against `schema`.
/// CAT-8.8 needs the model's summaries and is checked by the caller.
///
/// # Errors
///
/// The first failure, in the order of CAT-9.1: `WCAT-01` when the text is not JSON, `WCAT-02`
/// when `contract` or `contract_version` is not supported, `WCAT-01` when the document does not
/// satisfy its JSON Schema, `WCAT-03` when it violates a CAT-8 rule, and `WCAT-04` when its
/// `schema_document_id` is not the `document_id` of `schema`. Every code is a warning
/// ([`AdmissionCode::is_warning`]): the state document is then ignored.
pub fn admit_catalog_state(
    text: &str,
    schema: &CatalogSchemaDocument,
) -> Result<CatalogStateDocument, AdmissionError> {
    let contract = IncomingContract::CatalogState;
    let document = contract.parse_json(text)?;
    contract.check_contract(&document)?;
    contract.validate(&document)?;
    let state: CatalogStateDocument = decode(contract, &document)?;
    state_rules::check(&state, schema)
        .map_err(|violation| violation.into_error(contract, AdmissionCode::Wcat03))?;
    if state.schema_document_id != schema.document_id {
        return Err(contract.error(
            AdmissionCode::Wcat04,
            "/schema_document_id",
            format!(
                "the state is consistent with schema document `{}` / `{}`, but the schema in \
                 use is `{}` / `{}`",
                state.schema_document_id.origin,
                state.schema_document_id.snapshot,
                schema.document_id.origin,
                schema.document_id.snapshot
            ),
        ));
    }
    Ok(state)
}

/// The first violation of a CAT-8 rule.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RuleViolation {
    /// JSON Pointer (RFC 6901) of the offending value.
    pointer: String,
    /// English description naming the offending item.
    message: String,
}

impl RuleViolation {
    fn new(pointer: String, message: String) -> Self {
        Self { pointer, message }
    }

    fn into_error(self, contract: IncomingContract, code: AdmissionCode) -> AdmissionError {
        contract.error(code, &self.pointer, self.message)
    }
}

/// The index of the first item equal to an earlier one.
fn first_repeated<T: Eq + Hash>(items: impl IntoIterator<Item = T>) -> Option<usize> {
    let mut seen = HashSet::new();
    items.into_iter().position(|item| !seen.insert(item))
}
