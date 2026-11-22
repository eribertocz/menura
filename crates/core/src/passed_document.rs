//! What the audit record keeps of an input document as passed (PLN-6.3): its hash and the
//! version of its contract.

use menura_contracts::{Sha256Hash, hash_value};
use serde_json::Value;

/// The member of a contract document that holds its version.
const CONTRACT_VERSION: &str = "contract_version";

/// An input document as passed: its hash (PLN-6.3) and its `contract_version`, when it is a
/// JSON object whose `contract_version` is a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PassedDocument {
    /// The SHA-256 hash of its RFC 8785 canonical JSON, or of its raw bytes when it is not
    /// JSON.
    pub(crate) hash: Sha256Hash,
    /// Its `contract_version`, as written.
    pub(crate) contract_version: Option<String>,
}

impl PassedDocument {
    /// The document `bytes`, as passed.
    pub(crate) fn of(bytes: &[u8]) -> Self {
        match serde_json::from_slice::<Value>(bytes) {
            Ok(value) => Self {
                hash: hash_value(&value),
                contract_version: value
                    .get(CONTRACT_VERSION)
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            },
            Err(_) => Self {
                hash: menura_contracts::document_hash(bytes),
                contract_version: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use menura_contracts::document_hash;

    use super::PassedDocument;

    #[test]
    fn pln_6_3_a_json_document_keeps_its_hash_and_contract_version() {
        let bytes = br#"{"contract": "menura.query", "contract_version": "1.1.0"}"#;
        let document = PassedDocument::of(bytes);
        assert_eq!(document.hash, document_hash(bytes));
        assert_eq!(document.contract_version.as_deref(), Some("1.1.0"));
    }

    #[test]
    fn pln_6_3_a_document_that_is_not_json_has_no_contract_version() {
        let document = PassedDocument::of(b"{");
        assert_eq!(document.hash, document_hash(b"{"));
        assert_eq!(document.contract_version, None);
    }
}
