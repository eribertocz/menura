//! A derived-metric declaration of the `derived` extension (DER-3), as data types only.

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The `derived` of one metric whose keys and values hold in E0.c (DER-5.1.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedDeclaration {
    /// The JSON Pointer of the metric that declares it.
    pub metric: JsonPointer,
    /// The JSON Pointer of the `data` member of the metric's entry of ours, where every
    /// diagnostic about the declaration is located (E0C-2.6).
    pub data: JsonPointer,
    /// Where that `data` member starts in the document text.
    pub location: Location,
    /// The value of `on_division_by_zero`, when given; it is required only when the formula
    /// divides, which E0.f checks (`E0C-35`, DER-5.1.3).
    pub on_division_by_zero: Option<DivisionByZero>,
}

/// The values of `on_division_by_zero` (DER-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DivisionByZero {
    /// `null`: that division yields NULL.
    Null,
    /// `error`: execution fails with `RDER-01`.
    Error,
}
