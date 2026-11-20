//! Identifiers and references.

/// An identifier as written: either a regular identifier (`amount`) or a delimited one
/// (`"Order Amount"`).
///
/// The tree keeps the spelling and the kind; comparing names is reference resolution's job.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier {
    text: String,
    delimited: bool,
}

impl Identifier {
    /// A regular identifier: a letter or `_`, then letters, digits or `_`.
    ///
    /// Returns `None` when `text` does not have that form.
    ///
    /// ```
    /// use menura_model::expression::Identifier;
    /// assert!(Identifier::regular("amount_2").is_some());
    /// assert!(Identifier::regular("2amount").is_none());
    /// ```
    #[must_use]
    pub fn regular(text: &str) -> Option<Self> {
        let mut characters = text.chars();
        let first = characters.next()?;
        let starts_well = first.is_alphabetic() || first == '_';
        let continues_well =
            characters.all(|character| character.is_alphanumeric() || character == '_');
        (starts_well && continues_well).then(|| Self {
            text: text.to_owned(),
            delimited: false,
        })
    }

    /// A delimited identifier with the given content (quotes removed, `""` already undoubled).
    ///
    /// Returns `None` when `text` is empty.
    #[must_use]
    pub fn delimited(text: &str) -> Option<Self> {
        (!text.is_empty()).then(|| Self {
            text: text.to_owned(),
            delimited: true,
        })
    }

    /// The identifier's text, without quotes.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Whether the identifier was written between double quotes.
    #[must_use]
    pub fn is_delimited(&self) -> bool {
        self.delimited
    }
}

/// A reference to a column or field: `name` or `qualifier.name`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Reference {
    /// The dataset, when written: `dataset` in `dataset.field`.
    pub qualifier: Option<Identifier>,
    /// The column, field or metric name.
    pub name: Identifier,
}
