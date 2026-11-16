//! The contract hash of CON-2: SHA-256 over RFC 8785 canonical JSON, as 64 lowercase hex digits.

use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::canonical::canonical_json;

/// A SHA-256 hash written as exactly 64 lowercase hexadecimal digits, with no prefix (CON-2).
///
/// It deserializes only from a string of that form, so an invalid hash is unrepresentable.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Sha256Hash(String);

/// The text is not 64 lowercase hexadecimal digits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidSha256Hash;

impl fmt::Display for InvalidSha256Hash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a hash must be 64 lowercase hexadecimal digits")
    }
}

impl std::error::Error for InvalidSha256Hash {}

impl Sha256Hash {
    /// The 64 lowercase hexadecimal digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Sha256Hash {
    type Error = InvalidSha256Hash;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        if is_lowercase_hex(&text, 64, 64) {
            Ok(Self(text))
        } else {
            Err(InvalidSha256Hash)
        }
    }
}

impl From<Sha256Hash> for String {
    fn from(hash: Sha256Hash) -> Self {
        hash.0
    }
}

impl fmt::Display for Sha256Hash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Hashes `value` as CON-2 requires: SHA-256 of its RFC 8785 canonical JSON.
///
/// # Example
///
/// ```
/// use semantic_compiler_contracts::hash_value;
///
/// let hash = hash_value(&serde_json::json!({}));
/// assert_eq!(
///     hash.as_str(),
///     "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
/// );
/// ```
#[must_use]
pub fn hash_value(value: &Value) -> Sha256Hash {
    hash_bytes(canonical_json(value).as_bytes())
}

/// SHA-256 of raw bytes, as 64 lowercase hex digits.
pub(crate) fn hash_bytes(bytes: &[u8]) -> Sha256Hash {
    let digest = Sha256::digest(bytes);
    let mut text = String::with_capacity(64);
    for byte in digest {
        text.push(hex_digit(byte >> 4));
        text.push(hex_digit(byte & 0x0f));
    }
    Sha256Hash(text)
}

fn hex_digit(nibble: u8) -> char {
    char::from(if nibble < 10 {
        b'0' + nibble
    } else {
        b'a' + nibble - 10
    })
}

/// Whether `text` has between `minimum` and `maximum` characters, all in `[0-9a-f]`.
pub(crate) fn is_lowercase_hex(text: &str, minimum: usize, maximum: usize) -> bool {
    (minimum..=maximum).contains(&text.len())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::{Sha256Hash, hash_bytes};

    /// FIPS 180-2 test vectors.
    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            hash_bytes(b"").as_str(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hash_bytes(b"abc").as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_text_must_be_lowercase_hex() {
        assert!(Sha256Hash::try_from("A".repeat(64)).is_err());
        assert!(Sha256Hash::try_from("a".repeat(63)).is_err());
        assert!(Sha256Hash::try_from("a".repeat(64)).is_ok());
    }
}
