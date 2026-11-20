//! Where an element of a representation comes from: the query document or the model (E1-5
//! "Origin", E4-3).

use menura_model::internal_model::SourceLocation;

/// A line and a column in the text of a document, both from 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextPosition {
    line: usize,
    column: usize,
}

impl TextPosition {
    /// The position at `line` and `column`. Returns `None` when either is 0.
    ///
    /// ```
    /// use menura_representations::origin::TextPosition;
    /// assert!(TextPosition::new(1, 8).is_some());
    /// assert!(TextPosition::new(0, 8).is_none());
    /// ```
    #[must_use]
    pub fn new(line: usize, column: usize) -> Option<Self> {
        (line >= 1 && column >= 1).then_some(Self { line, column })
    }

    /// The line, from 1.
    #[must_use]
    pub fn line(self) -> usize {
        self.line
    }

    /// The column, from 1.
    #[must_use]
    pub fn column(self) -> usize {
        self.column
    }
}

/// Where an element is written in the query document.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryLocation {
    pointer: String,
    position: Option<TextPosition>,
}

impl QueryLocation {
    /// The element at the JSON Pointer `pointer`, at `position` in a text such as the SQL of
    /// the SQL form when it is known.
    #[must_use]
    pub fn new(pointer: &str, position: Option<TextPosition>) -> Self {
        Self {
            pointer: pointer.to_owned(),
            position,
        }
    }

    /// The JSON Pointer (RFC 6901) of the element in the query document.
    #[must_use]
    pub fn pointer(&self) -> &str {
        &self.pointer
    }

    /// The line and column in the text, when known.
    #[must_use]
    pub fn position(&self) -> Option<TextPosition> {
        self.position
    }
}

/// The input element a plan node comes from (E4-3): a query element, or a model element such
/// as a dataset or a row policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Origin {
    /// An element of the query document.
    Query(QueryLocation),
    /// An element of the model document.
    Model(SourceLocation),
}
