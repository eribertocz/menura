//! Stage E0: loads and validates a semantic model.

#![forbid(unsafe_code)]

/// Throwaway function that clippy rejects (`map_identity`).
#[must_use]
pub fn throwaway() -> Option<u8> {
    Some(1).map(|value| value)
}
