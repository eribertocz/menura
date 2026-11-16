//! Stage E0.a: reads the model document text into a located data tree (E0A-1, MDL-1.2).
//!
//! The document is read as YAML 1.1 safe loading reads it, with duplicate keys an error
//! (E0A-2, DEV-1). The reader receives the text and never reads files.
//!
//! Reading runs in three steps, each in its own module: decoding the UTF-8 text, composing the
//! parser events of the single document into a raw tree, and constructing the data tree from it.

mod compose;
mod construct;
mod error;
mod key;
mod numbers;
mod resolve;
mod scalar;
mod timestamp;
mod tree;

pub use error::{ReadError, ReadErrorCode};
pub use tree::{Entry, Integer, Location, Node, Value};

/// Reads a model document into its located data tree.
///
/// JSON documents are read as YAML (E0A-2.2). An empty document reads as a null node.
///
/// # Errors
///
/// Returns a [`ReadError`] with the code of E0A-3: `E0A-01` when the bytes are not UTF-8, `E0A-02`
/// when the YAML cannot be parsed, `E0A-03` when there is more than one document, `E0A-04` for a
/// repeated key and `E0A-05` for a tag or value that cannot be constructed.
///
/// # Example
///
/// ```
/// use semantic_compiler_e0_load::read::{ReadErrorCode, Value, read_document};
///
/// let document = read_document(b"enabled: yes\nsince: 2026-11-16\n")?;
/// let Value::Mapping(entries) = &document.value else { unreachable!() };
/// assert_eq!(entries[0].value.value, Value::Boolean(true));
/// assert_eq!(entries[1].value.value, Value::String("2026-11-16".to_owned()));
///
/// let error = read_document(b"a: 1\na: 2\n").unwrap_err();
/// assert_eq!(error.code, ReadErrorCode::DuplicateKey);
/// # Ok::<(), semantic_compiler_e0_load::read::ReadError>(())
/// ```
pub fn read_document(bytes: &[u8]) -> Result<Node, ReadError> {
    let text = decode(bytes)?;
    let raw = compose::compose(text)?;
    construct::construct(&raw)
}

/// The text of the document, or `E0A-01` at the first byte that is not UTF-8 (E0A-2.1).
fn decode(bytes: &[u8]) -> Result<&str, ReadError> {
    std::str::from_utf8(bytes).map_err(|error| {
        let valid = bytes
            .get(..error.valid_up_to())
            .and_then(|prefix| std::str::from_utf8(prefix).ok())
            .unwrap_or_default();
        let last_line = valid.rsplit('\n').next().unwrap_or_default();
        let location = Location {
            line: valid.matches('\n').count() + 1,
            column: last_line.chars().count() + 1,
        };
        ReadError::new(
            ReadErrorCode::InvalidEncoding,
            location,
            "the document is not valid UTF-8",
        )
    })
}
