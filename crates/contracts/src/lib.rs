//! Contract documents exchanged between modules and the types that represent them.
//!
//! - [`types`]: a Rust type for every contract document, serialized exactly as its JSON Schema.
//! - [`validate_structure`]: validation of a document against its published JSON Schema.
//! - [`IncomingContract`]: admission of incoming documents with each contract's error codes.
//! - [`canonical_json`] and [`hash_value`]: RFC 8785 canonical JSON and the CON-2 hash.
//! - [`value_encoding`]: the CON-4 canonical string forms.

#![forbid(unsafe_code)]

mod admission;
mod canonical;
mod contract;
mod hash;
mod ordering;
mod schema;
pub mod types;
pub mod value_encoding;

pub use admission::{AdmissionCode, AdmissionError, IncomingContract, check_query_form, decode};
pub use canonical::canonical_json;
pub use contract::{ContractKind, ContractVersion};
pub use hash::{InvalidSha256Hash, Sha256Hash, hash_value};
pub use ordering::compare_names;
pub use schema::{SchemaViolation, validate_structure};
pub use value_encoding::encode_integer;
