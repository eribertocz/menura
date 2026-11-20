//! The names of output columns and of plan columns.

use crate::query_tree::QueryReference;

/// The name of a column of the query result, as written (E1-5.1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputName(String);

impl OutputName {
    /// The output name `text`, such as an alias as written.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The output name of an element without an alias: the last part of its reference as
    /// written, never normalized, without the quotes of a delimited part (E1-5.1).
    ///
    /// ```
    /// use menura_representations::names::OutputName;
    /// use menura_representations::query_tree::{QueryReference, ReferencePart};
    /// let reference = QueryReference::new(vec![
    ///     ReferencePart::Regular("orders".to_owned()),
    ///     ReferencePart::Delimited("Net Amount".to_owned()),
    /// ])
    /// .expect("two parts");
    /// assert_eq!(OutputName::from_reference(&reference).as_str(), "Net Amount");
    /// ```
    #[must_use]
    pub fn from_reference(reference: &QueryReference) -> Self {
        Self::new(reference.last_part().map_or("", |part| part.text()))
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The name of a column produced by a plan node (E4-3).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnName(String);

impl ColumnName {
    /// The column name `text`.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The name as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
