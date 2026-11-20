//! Values shared by the unit tests of this crate.

use menura_model::expression::{Expression, Identifier, Literal, Reference};
use menura_model::internal_model::{NormalizedName, SourceLocation};

use menura_model::expression::{DateValue, TimeOfDay, TimestampValue};

use crate::origin::{Origin, QueryLocation, TextPosition};
use crate::query_tree::{QueryReference, ReferencePart};
use crate::time::{QueryTime, ReferenceTime, TimeZoneName, UtcOffset};

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

/// The query reference whose parts are the regular identifiers `parts`, as written.
pub(crate) fn query_reference(parts: &[&str]) -> QueryReference {
    QueryReference::new(
        parts
            .iter()
            .map(|part| ReferencePart::Regular((*part).to_owned()))
            .collect(),
    )
    .expect("at least one part")
}

/// The time zone `Europe/Madrid` and the reference time 2026-03-31T10:00:00+02:00.
pub(crate) fn madrid_time() -> QueryTime {
    let local = TimestampValue {
        date: DateValue::new(2026, 3, 31).expect("a date"),
        time: TimeOfDay::new(10, 0, 0, "").expect("a time of day"),
    };
    let offset = UtcOffset::from_minutes(120).expect("an offset");
    QueryTime {
        time_zone: TimeZoneName::new("Europe/Madrid"),
        reference_date: Some(local.date),
        reference_time: Some(ReferenceTime { local, offset }),
    }
}
