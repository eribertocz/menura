//! The result of reference resolution, as data types only.

use super::diagnostic::{ReferenceCode, ReferenceDiagnostic};
use crate::pointer::JsonPointer;

/// The references of a model that loads: the elements a missing reference or an unlisted
/// reference form makes unusable, and their warnings.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct References {
    /// One entry per unusable element, in document order.
    pub unusable: Vec<UnusableElement>,
    /// One diagnostic per missing reference or unlisted reference form, in document order then
    /// by code (E0-4.3). Each is reported as `W0D-01` or `W0F-01` ([`ReferenceCode::load_code`]).
    pub warnings: Vec<ReferenceDiagnostic>,
}

/// Why the model does not load: the rejecting diagnostics, and the warnings found with them, so
/// the compile error document can report both (E0-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceRejection {
    /// `E0D-01`, `E0D-02` and `E0D-08` diagnostics, in document order then by code; never
    /// empty.
    pub errors: Vec<ReferenceDiagnostic>,
    /// The other diagnostics, as in [`References::warnings`].
    pub warnings: Vec<ReferenceDiagnostic>,
}

/// What kind of element a missing reference makes unusable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElementKind {
    /// A dataset whose key cannot be resolved.
    Dataset,
    /// A field whose expression reads a missing column or has an unlisted reference form.
    Field,
    /// A metric that refers to a missing dataset or field, or has an unlisted reference form.
    Metric,
    /// A relationship that refers to a missing dataset or column.
    Relationship,
}

/// An element marked unusable by a missing reference or an unlisted reference form (E0-4.1).
///
/// Only the element that holds the reference is recorded. What follows from it (E0-4.2: the
/// fields, metrics and relationships of an unusable dataset) is derived when the internal model
/// is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnusableElement {
    /// The kind of element.
    pub kind: ElementKind,
    /// Its JSON Pointer, such as `/semantic_model/0/metrics/2`.
    pub pointer: JsonPointer,
    /// The query error of a query that names it: the lowest code among its missing
    /// references, the first that E2-5 row 12a raises.
    pub query_error: ReferenceCode,
}
