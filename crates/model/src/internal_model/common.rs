//! The values every part of the internal model shares: normalized names, source locations,
//! descriptions, types and codes.

use semantic_compiler_contracts::types::common::LogicalType;

use crate::json::JsonForm;

/// A name after normalization (E0D-1.1, E0D-1.4): a regular identifier in upper case, a
/// delimited one exactly as written without its quotes. Ordering compares Unicode code points
/// (CON-5).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NormalizedName(String);

impl NormalizedName {
    /// The name whose normalized text is `text`; the caller has normalized it.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The normalized text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A reference `dataset.field` to a field of the model, with normalized names.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FieldPath {
    /// The dataset.
    pub dataset: NormalizedName,
    /// The field of that dataset.
    pub field: NormalizedName,
}

/// Where an element is written in the model document, for diagnostics and explanations
/// (E0H-1.5). Never part of a hash (E0H-3.4).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    /// The JSON Pointer of the element, such as `/semantic_model/0/datasets/1`.
    pub pointer: String,
    /// The line where it starts, from 1.
    pub line: usize,
    /// The column where it starts, from 1.
    pub column: usize,
}

/// The `description`, `label` and `ai_context` of an element, as written (E0H-2).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Descriptions {
    /// `description`, when written.
    pub description: Option<String>,
    /// `label`, when written.
    pub label: Option<String>,
    /// `ai_context`, when written: a string or an object, kept as its JSON value.
    pub ai_context: Option<JsonForm>,
}

/// The result type of an expression: a logical type of the catalog schema, or the type of the
/// literal `NULL`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ValueType {
    /// The type of the literal `NULL`.
    Null,
    /// A logical type (CAT-4.1).
    Logical(LogicalType),
}

/// The result type of an expression and whether it can be NULL (E0F-7.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypedValue {
    /// The result type.
    pub value_type: ValueType,
    /// Whether the value can be NULL.
    pub nullable: bool,
}

/// A diagnostic code as the specification writes it, such as `E0D-03` or `W0F-01`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    /// The code `code`.
    #[must_use]
    pub fn new(code: &str) -> Self {
        Self(code.to_owned())
    }

    /// The code as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
