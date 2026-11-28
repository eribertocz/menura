//! The range shape of a `validity` population (VAL-1): whether its end is inclusive or
//! exclusive, and the value of `valid_to` that means open-ended.

use crate::expression::DateValue;

/// The values of `valid_to_bound` (POP-2, VAL-1.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ValidToBound {
    /// `inclusive`, the default: `valid_to` is the last value the row covers.
    #[default]
    Inclusive,
    /// `exclusive`: `valid_to` is the first value the row does not cover.
    Exclusive,
}

impl ValidToBound {
    /// The value as the extension writes it.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inclusive => "inclusive",
            Self::Exclusive => "exclusive",
        }
    }
}

/// The `open_end` of a `validity` population (VAL-1.2): a value of the logical type of its
/// `valid_to`, which E0.f checked against its CON-4 form (VAL-1.5).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OpenEnd {
    /// A value of a `date` `valid_to`.
    Date(DateValue),
    /// A value of a `timestamp` `valid_to`.
    Timestamp(OpenEndTimestamp),
}

/// A `timestamp` value of `open_end`, in its CON-4 parts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OpenEndTimestamp {
    /// The date part.
    pub date: DateValue,
    /// The time part as CON-4 writes it: `HH:MM:SS`, then `.` and the fraction digits without
    /// trailing zeros when they are not zero.
    pub time: String,
    /// Whether it is an instant, written in UTC with a final `Z`, or a wall-clock value.
    pub with_time_zone: bool,
}

impl OpenEnd {
    /// The value in its CON-4 form, such as `9999-12-31` or `9999-12-31T00:00:00Z`.
    #[must_use]
    pub fn con4_text(&self) -> String {
        match self {
            Self::Date(date) => date_text(*date),
            Self::Timestamp(timestamp) => {
                let zone = if timestamp.with_time_zone { "Z" } else { "" };
                format!("{}T{}{zone}", date_text(timestamp.date), timestamp.time)
            }
        }
    }
}

/// `YYYY-MM-DD` (CON-4).
fn date_text(date: DateValue) -> String {
    format!("{:04}-{:02}-{:02}", date.year(), date.month(), date.day())
}
