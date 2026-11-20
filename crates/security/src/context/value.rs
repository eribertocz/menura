//! Typed attribute values of an admitted security context (SEC-3.3): one variant per declared
//! type of SCY-2.2, so no string stands for a number or a date.

use std::cmp::Ordering;

use menura_contracts::value_encoding::is_canonical_date;

/// The value of a declared attribute, checked against its declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmittedValue {
    /// The value of a scalar attribute.
    Scalar(AdmittedScalar),
    /// The items of a `list<…>` attribute, in the order written; possibly empty.
    List(Vec<AdmittedScalar>),
}

/// One value of a declared type (SCY-2.2).
///
/// The derived order compares two values of the same variant by value, as SEC-3.6 sorts list
/// items: text in CON-5 order (Unicode code points), integers and decimals numerically, dates
/// chronologically, and `false` before `true`. The items of one list always share a variant.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdmittedScalar {
    /// `text`.
    Text(String),
    /// `integer`: a signed 64-bit integer.
    Integer(i64),
    /// `decimal(p,s)`.
    Decimal(DecimalValue),
    /// `date`.
    Date(DateValue),
    /// `boolean`.
    Boolean(bool),
}

/// An exact decimal: `unscaled × 10^(−scale)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecimalValue {
    unscaled: i128,
    scale: u8,
}

impl DecimalValue {
    /// The decimal `unscaled × 10^(−scale)`.
    ///
    /// ```
    /// use menura_security::DecimalValue;
    ///
    /// assert_eq!(DecimalValue::new(-1250, 2).to_string(), "-12.50");
    /// ```
    #[must_use]
    pub fn new(unscaled: i128, scale: u8) -> Self {
        Self { unscaled, scale }
    }

    /// The digits of the value, without the point.
    #[must_use]
    pub fn unscaled(self) -> i128 {
        self.unscaled
    }

    /// The number of digits after the point.
    #[must_use]
    pub fn scale(self) -> u8 {
        self.scale
    }
}

impl PartialOrd for DecimalValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DecimalValue {
    /// Numeric order for values of one scale, the only values a list holds; the scale breaks
    /// ties otherwise, so the order stays total.
    fn cmp(&self, other: &Self) -> Ordering {
        self.scale
            .cmp(&other.scale)
            .then(self.unscaled.cmp(&other.unscaled))
    }
}

impl std::fmt::Display for DecimalValue {
    /// The CON-4 canonical form: optional minus, the integer digits without leading zeros, and
    /// `.` with exactly `scale` digits when `scale` is not zero.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sign = if self.unscaled < 0 { "-" } else { "" };
        let digits = format!(
            "{:0width$}",
            self.unscaled.unsigned_abs(),
            width = usize::from(self.scale) + 1
        );
        let (integer, fraction) = digits.split_at(digits.len() - usize::from(self.scale));
        if fraction.is_empty() {
            write!(formatter, "{sign}{integer}")
        } else {
            write!(formatter, "{sign}{integer}.{fraction}")
        }
    }
}

/// A valid date of the proleptic Gregorian calendar. The derived order is chronological.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateValue {
    year: u16,
    month: u8,
    day: u8,
}

impl DateValue {
    /// The date `year-month-day`, or `None` when it is not a date of the proleptic Gregorian
    /// calendar with a four-digit year.
    ///
    /// ```
    /// use menura_security::DateValue;
    ///
    /// assert_eq!(DateValue::new(2026, 2, 1).unwrap().to_string(), "2026-02-01");
    /// assert_eq!(DateValue::new(2026, 2, 30), None);
    /// ```
    #[must_use]
    pub fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        let date = Self { year, month, day };
        is_canonical_date(&date.to_string()).then_some(date)
    }
}

impl std::fmt::Display for DateValue {
    /// The CON-4 canonical form, `YYYY-MM-DD`.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}
