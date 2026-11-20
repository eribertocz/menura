//! Contract documents exchanged between modules and the types that represent them.
//!
//! - [`types`]: a Rust type for every contract document, serialized exactly as its JSON Schema.
//! - [`validate_structure`]: validation of a document against its published JSON Schema, and
//!   [`validate_against_schema`] and [`schema_violations`]: the same evaluator for a schema the
//!   caller provides, with the first violation or all of them.
//! - [`IncomingContract`]: admission of incoming documents with each contract's error codes.
//! - [`admit_catalog_schema`] and [`admit_catalog_state`]: admission of the catalog documents in
//!   the order of CAT-9.1, with the CAT-8 rules, and [`schema_former_name_warning`] and
//!   [`state_former_name_warning`]: the `WCAT-05` warning of an admitted catalog document that
//!   uses the former contract name (CON-0.5).
//! - [`canonical_json`] and [`hash_value`]: RFC 8785 canonical JSON and the CON-2 hash.
//! - [`normalize_unquoted_identifier`]: E0D-1.1 normalization with the CON-5 simple mapping.
//! - [`value_encoding`]: the CON-4 canonical string forms.

#![forbid(unsafe_code)]

mod admission;
mod canonical;
mod catalog_admission;
mod contract;
mod hash;
mod identifier;
mod ordering;
mod schema;
pub mod types;
pub mod value_encoding;

pub use admission::{AdmissionCode, AdmissionError, IncomingContract, check_query_form, decode};
pub use canonical::canonical_json;
pub use catalog_admission::{
    admit_catalog_schema, admit_catalog_state, schema_former_name_warning,
    state_former_name_warning,
};
pub use contract::{ContractKind, ContractVersion, FORMER_NAMESPACE, NAMESPACE};
pub use hash::{InvalidSha256Hash, Sha256Hash, hash_value};
pub use identifier::normalize_unquoted_identifier;
pub use ordering::compare_names;
pub use schema::{
    SchemaViolation, SchemaViolationKind, schema_violations, validate_against_schema,
    validate_structure,
};
pub use value_encoding::encode_integer;
