//! The names of output columns and of plan columns.

use menura_model::expression::Reference;

/// The name of a column of the query result, as written (E1-5.1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutputName(String);

impl OutputName {
    /// The output name `text`, such as an alias as written.
    #[must_use]
    pub fn new(text: &str) -> Self {
        Self(text.to_owned())
    }

    /// The output name of an element without an alias: the last part of its reference
    /// (E1-5.1).
    ///
    /// ```
    /// use menura_model::expression::{Identifier, Reference};
    /// use menura_representations::names::OutputName;
    /// let reference = Reference {
    ///     qualifier: Identifier::regular("orders"),
    ///     name: Identifier::regular("region").expect("a regular identifier"),
    /// };
    /// assert_eq!(OutputName::from_reference(&reference).as_str(), "region");
    /// ```
    #[must_use]
    pub fn from_reference(reference: &Reference) -> Self {
        Self::new(reference.name.text())
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
