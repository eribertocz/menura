//! A declaration of the `temporal` extension (VAL-2.1), as data types only.

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The `temporal` entry of a relationship whose shape is valid (VAL-2.1).
///
/// Names are kept as written: `from_time` is a declared name (E0D-1.4) of a field of the
/// relationship's `from` dataset, and `population` the name of a population of its `to`
/// dataset, both resolved by E0.d (VAL-2.2, VAL-2.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalDeclaration {
    /// The JSON Pointer of the relationship that carries the entry.
    pub relationship: JsonPointer,
    /// The JSON Pointer of the `data` member of the relationship's entry of ours, where every
    /// diagnostic about the declaration is located (E0C-2.6).
    pub data: JsonPointer,
    /// Where that `data` member starts in the document text.
    pub location: Location,
    /// The value of `from_time`.
    pub from_time: String,
    /// The value of `population`.
    pub population: String,
}
