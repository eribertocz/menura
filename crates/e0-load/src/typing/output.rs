//! The result of typing, as data types only.

use super::diagnostic::TypingDiagnostic;
use crate::pointer::JsonPointer;
use menura_expressions::typing::Typed;
use menura_expressions::typing::TypingCode;

/// The types of a model's fields and metrics, the elements a typing problem makes unusable,
/// and their warnings. Typing never rejects the model: every `E0F` problem makes its element
/// unusable (E0-4 table).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Typing {
    /// The result type and nullability of every field, dataset metric and model metric that
    /// could be typed, in document order (E0F-7.1).
    pub elements: Vec<TypedElement>,
    /// One entry per element a typing problem makes unusable, in document order.
    pub unusable: Vec<UnusableElement>,
    /// One diagnostic per typing problem, in document order then by code (E0-4.3). Each is
    /// reported as `W0F-01`.
    pub warnings: Vec<TypingDiagnostic>,
}

/// What kind of element typing is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElementKind {
    /// A field of a dataset.
    Field,
    /// A metric of the semantic model.
    Metric,
    /// A dataset-scoped metric of the development format (section 4.5).
    DatasetMetric,
    /// A relationship, whose key pairs are typed (E0F-4.1).
    Relationship,
}

/// The result type of a field or metric expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedElement {
    /// [`ElementKind::Field`], [`ElementKind::Metric`] or [`ElementKind::DatasetMetric`].
    pub kind: ElementKind,
    /// Its JSON Pointer, such as `/semantic_model/0/metrics/2`.
    pub pointer: JsonPointer,
    /// Its result type and whether it can be NULL.
    pub typed: Typed,
}

/// An element marked unusable by a typing problem (E0-4.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnusableElement {
    /// The kind of element.
    pub kind: ElementKind,
    /// Its JSON Pointer.
    pub pointer: JsonPointer,
    /// The query error of a query that names it: the lowest code among its problems.
    pub query_error: TypingCode,
}
