//! A JSON value as plain data: the form in which this crate defines the normalized trees that
//! hashes cover (E0H-3.4).
//!
//! The crate has no JSON library: a crate that hashes converts this form into its own JSON
//! values and hashes those (CON-2). Values follow CON-4: exact values are strings in canonical
//! form, small integers and floats are numbers.

use std::collections::BTreeMap;

/// A JSON value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JsonForm {
    /// `null`.
    Null,
    /// `true` or `false`.
    Boolean(bool),
    /// An integer within ±(2^53 − 1), written as a JSON number (CON-4).
    Integer(i64),
    /// A float, as the text of a JSON number such as `1.5e3`; the hashing crate reads it as an
    /// IEEE-754 double (CON-4).
    Number(String),
    /// A string.
    Text(String),
    /// An array, in order.
    List(Vec<JsonForm>),
    /// An object; its members are ordered by name, as canonical JSON orders them anyway.
    Object(BTreeMap<String, JsonForm>),
}

impl JsonForm {
    /// An object with `members`.
    ///
    /// ```
    /// use semantic_compiler_model::json::JsonForm;
    /// let object = JsonForm::object([("node", JsonForm::text("current_date"))]);
    /// assert_eq!(object, JsonForm::object([("node", JsonForm::text("current_date"))]));
    /// ```
    #[must_use]
    pub fn object<const COUNT: usize>(members: [(&str, JsonForm); COUNT]) -> Self {
        Self::Object(
            members
                .into_iter()
                .map(|(name, value)| (name.to_owned(), value))
                .collect(),
        )
    }

    /// A string.
    #[must_use]
    pub fn text(text: &str) -> Self {
        Self::Text(text.to_owned())
    }

    /// `value`, or `null` when it is `None`.
    #[must_use]
    pub fn optional(value: Option<JsonForm>) -> Self {
        value.unwrap_or(Self::Null)
    }

    /// An array of `items`.
    #[must_use]
    pub fn list<Item>(items: impl IntoIterator<Item = Item>, form: impl Fn(Item) -> Self) -> Self {
        Self::List(items.into_iter().map(form).collect())
    }
}
