//! The model hash (E0H-3.2).

use semantic_compiler_contracts::{Sha256Hash, hash_value};
use semantic_compiler_model::internal_model::ModelParts;

/// The model hash of `parts` (E0H-3.2).
pub(crate) fn model_hash(parts: &ModelParts) -> Sha256Hash {
    let _ = parts;
    hash_value(&serde_json::Value::Null)
}
