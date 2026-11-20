//! Values shared by the unit tests of this crate.

use menura_model::expression::{Expression, Identifier, Literal, Reference};
use menura_model::internal_model::{NormalizedName, SourceLocation};

use crate::origin::{Origin, QueryLocation, TextPosition};

/// A normalized name.
pub(crate) fn name(text: &str) -> NormalizedName {
    NormalizedName::new(text)
}

/// A location in the query document, without a text position.
pub(crate) fn at(pointer: &str) -> QueryLocation {
    QueryLocation::new(pointer, None)
}

/// A location in the model document.
pub(crate) fn in_model(pointer: &str) -> Origin {
    Origin::Model(SourceLocation {
        pointer: pointer.to_owned(),
        line: 1,
        column: 1,
    })
}

/// A location in the SQL text of the query document.
pub(crate) fn in_sql(line: usize, column: usize) -> QueryLocation {
    QueryLocation::new("/sql", TextPosition::new(line, column))
}

/// The reference `qualifier.name`, both regular identifiers.
pub(crate) fn reference(qualifier: &str, field: &str) -> Reference {
    Reference {
        qualifier: Identifier::regular(qualifier),
        name: Identifier::regular(field).expect("a regular identifier"),
    }
}

/// The literal `TRUE`.
pub(crate) fn true_literal() -> Expression {
    Expression::Literal(Literal::Boolean(true))
}
