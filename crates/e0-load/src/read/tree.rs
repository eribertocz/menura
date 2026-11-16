//! The located data tree that reading produces (E0A-1, E0A-2.7), as data types only.
//!
//! Building the tree is done by [`super::read_document`].

use std::fmt;
use std::sync::Arc;

/// Where a node starts in the document text (E0A-2.7).
///
/// Both numbers start at 1. Columns count Unicode scalar values, not bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Location {
    /// The line, starting at 1.
    pub line: usize,
    /// The column, starting at 1.
    pub column: usize,
}

/// One node of the data tree, with the location of its content.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The value of the node.
    pub value: Value,
    /// Where the content of the node starts. An alias expands to the anchored node, which keeps
    /// the location where its content is written.
    pub location: Location,
}

/// The value of a node, as YAML 1.1 safe loading constructs it (E0A-2.2).
///
/// Dates and timestamps are already converted to their ISO 8601 text (E0A-2.8), so every value
/// is a JSON value, except the non-finite floats `.inf` and `.nan`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`, `~` or an empty scalar.
    Null,
    /// A YAML 1.1 boolean, such as `true`, `yes` or `off`.
    Boolean(bool),
    /// A YAML 1.1 integer, of any size.
    Integer(Integer),
    /// A YAML 1.1 float.
    Float(f64),
    /// A string, including dates and timestamps in ISO 8601 text.
    String(String),
    /// A sequence. Its items are shared, so expanding an alias does not copy them.
    Sequence(Arc<[Node]>),
    /// A mapping, with its merge keys expanded and its keys in first-insertion order. Its entries
    /// are shared, so expanding an alias does not copy them.
    Mapping(Arc<[Entry]>),
}

/// One entry of a mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The key. It is never a sequence or a mapping.
    pub key: Node,
    /// The value.
    pub value: Node,
}

/// An integer of any size, kept as its canonical decimal digits.
///
/// # Example
///
/// ```
/// use semantic_compiler_e0_load::read::{Value, read_document};
///
/// let document = read_document(b"0x1F")?;
/// let Value::Integer(integer) = document.value else { unreachable!() };
/// assert_eq!(integer.to_string(), "31");
/// # Ok::<(), semantic_compiler_e0_load::read::ReadError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Integer {
    negative: bool,
    /// Decimal digits, most significant first, without leading zeros; zero is `"0"`.
    magnitude: String,
}

impl Integer {
    /// Builds an integer from its sign and canonical decimal digits. Zero is never negative.
    pub(crate) fn new(negative: bool, magnitude: String) -> Self {
        let negative = negative && magnitude != "0";
        Self {
            negative,
            magnitude,
        }
    }

    /// Whether the integer is below zero.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// The decimal digits of the absolute value, without leading zeros.
    #[must_use]
    pub fn magnitude(&self) -> &str {
        &self.magnitude
    }
}

impl fmt::Display for Integer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.negative {
            formatter.write_str("-")?;
        }
        formatter.write_str(&self.magnitude)
    }
}
