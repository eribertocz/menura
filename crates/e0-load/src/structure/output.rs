//! The result of structure validation, as data types only.

use super::diagnostic::StructureDiagnostic;
use crate::pointer::JsonPointer;
use crate::read::Node;

/// A document whose structure is valid: the semantic model to compile, the `ANSI_SQL` variant of
/// each of its expressions, and the load warnings.
#[derive(Debug, Clone, PartialEq)]
pub struct Structure<'document> {
    /// The semantic model to compile (E0B-3.2); the others are ignored.
    pub semantic_model: SelectedModel<'document>,
    /// One entry per field and per metric of the selected model, in document order.
    pub expressions: Vec<ElementExpression<'document>>,
    /// `W0B-01` and `W0B-02` warnings, in document order then by code (E0-4.3).
    pub warnings: Vec<StructureDiagnostic>,
}

/// The semantic model selected by the load option `model_name` (E0B-3.2, DEV-7).
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedModel<'document> {
    /// Its index in `semantic_model`.
    pub index: usize,
    /// Its JSON Pointer, such as `/semantic_model/0`.
    pub pointer: JsonPointer,
    /// Its node in the data tree.
    pub node: &'document Node,
}

/// What kind of element an expression belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ElementKind {
    /// A field of a dataset.
    Field,
    /// A metric of the semantic model.
    Metric,
}

/// The expression of one field or metric (E0B-3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementExpression<'document> {
    /// Whether the element is a field or a metric.
    pub kind: ElementKind,
    /// The JSON Pointer of the element, such as `/semantic_model/0/datasets/2/fields/1`.
    pub pointer: JsonPointer,
    /// Its `ANSI_SQL` variant, or why it has none to compile.
    pub variant: CompiledVariant<'document>,
}

impl ElementExpression<'_> {
    /// The query error code of a query that names the element, when it is unusable (E0-4.1).
    pub const UNUSABLE_QUERY_ERROR: &'static str = "E0B-08";

    /// Whether the element can be compiled.
    #[must_use]
    pub fn is_usable(&self) -> bool {
        matches!(self.variant, CompiledVariant::AnsiSql(_))
    }
}

/// The variant of an expression that is compiled (E0B-3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompiledVariant<'document> {
    /// The text of the single `ANSI_SQL` variant.
    AnsiSql(&'document str),
    /// No `ANSI_SQL` variant, or more than one: the element is unusable (`W0B-02`, OSS-2.2), and a
    /// query that names it fails with [`ElementExpression::UNUSABLE_QUERY_ERROR`].
    Unusable,
}
