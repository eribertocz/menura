//! Literal values of the accepted subset (E0F-2.2).

/// A constant written in an expression.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    /// `NULL`.
    Null,
    /// `TRUE` or `FALSE`.
    Boolean(bool),
    /// A text literal, with `''` already undoubled.
    Text(String),
    /// An exact number such as `12` or `0.50`.
    Exact(ExactNumber),
    /// An approximate number such as `1.5E3`.
    Approximate(ApproximateNumber),
    /// `DATE 'YYYY-MM-DD'`.
    Date(DateValue),
    /// `TIMESTAMP 'YYYY-MM-DD HH:MM:SS[.fraction]'`.
    Timestamp(TimestampValue),
}

/// An unsigned exact number: integer digits and fraction digits.
///
/// Leading zeros of the integer part are not kept (`007` equals `7`, `.5` equals `0.5`), but
/// every fraction digit is, because the number of fraction digits is the literal's scale
/// (`0.50` differs from `0.5`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExactNumber {
    integer_digits: String,
    fraction_digits: String,
}

impl ExactNumber {
    /// Builds a number from its digits before and after the decimal point.
    ///
    /// Returns `None` when a part holds anything but ASCII digits or when both are empty.
    ///
    /// ```
    /// use semantic_compiler_model::expression::ExactNumber;
    /// let number = ExactNumber::new("007", "50").unwrap();
    /// assert_eq!((number.integer_digits(), number.fraction_digits()), ("7", "50"));
    /// ```
    #[must_use]
    pub fn new(integer_digits: &str, fraction_digits: &str) -> Option<Self> {
        let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
        if !all_digits(integer_digits)
            || !all_digits(fraction_digits)
            || (integer_digits.is_empty() && fraction_digits.is_empty())
        {
            return None;
        }
        let trimmed = integer_digits.trim_start_matches('0');
        let integer_digits = if trimmed.is_empty() { "0" } else { trimmed };
        Some(Self {
            integer_digits: integer_digits.to_owned(),
            fraction_digits: fraction_digits.to_owned(),
        })
    }

    /// The integer digits, without leading zeros; `"0"` when the integer part is zero.
    #[must_use]
    pub fn integer_digits(&self) -> &str {
        &self.integer_digits
    }

    /// The fraction digits as written; empty for an integer.
    #[must_use]
    pub fn fraction_digits(&self) -> &str {
        &self.fraction_digits
    }
}

/// An approximate number: `mantissa × 10^exponent`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApproximateNumber {
    /// The digits before the exponent.
    pub mantissa: ExactNumber,
    /// The power of ten.
    pub exponent: i32,
}

/// A calendar date of the proleptic Gregorian calendar, years 1 to 9999.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateValue {
    year: u16,
    month: u8,
    day: u8,
}

impl DateValue {
    /// Builds a date. Returns `None` when the year is outside 1–9999 or the day does not exist.
    ///
    /// ```
    /// use semantic_compiler_model::expression::DateValue;
    /// assert!(DateValue::new(2024, 2, 29).is_some());
    /// assert!(DateValue::new(2026, 2, 29).is_none());
    /// ```
    #[must_use]
    pub fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        let valid = (1..=9999).contains(&year)
            && (1..=12).contains(&month)
            && day >= 1
            && day <= days_in_month(year, month);
        valid.then_some(Self { year, month, day })
    }

    /// The year, 1–9999.
    #[must_use]
    pub fn year(self) -> u16 {
        self.year
    }

    /// The month, 1–12.
    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    /// The day of the month, from 1.
    #[must_use]
    pub fn day(self) -> u8 {
        self.day
    }
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// A time of day with up to nine fraction digits of a second.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
    second: u8,
    fraction_digits: String,
}

impl TimeOfDay {
    /// Builds a time of day. Returns `None` when a field is out of range, or when the fraction
    /// has more than nine digits or anything but ASCII digits.
    #[must_use]
    pub fn new(hour: u8, minute: u8, second: u8, fraction_digits: &str) -> Option<Self> {
        let valid = hour < 24
            && minute < 60
            && second < 60
            && fraction_digits.len() <= 9
            && fraction_digits.bytes().all(|byte| byte.is_ascii_digit());
        valid.then(|| Self {
            hour,
            minute,
            second,
            fraction_digits: fraction_digits.to_owned(),
        })
    }

    /// The hour, 0–23.
    #[must_use]
    pub fn hour(&self) -> u8 {
        self.hour
    }

    /// The minute, 0–59.
    #[must_use]
    pub fn minute(&self) -> u8 {
        self.minute
    }

    /// The second, 0–59.
    #[must_use]
    pub fn second(&self) -> u8 {
        self.second
    }

    /// The fraction digits of the second as written, at most nine; empty when none.
    #[must_use]
    pub fn fraction_digits(&self) -> &str {
        &self.fraction_digits
    }
}

/// A timestamp without time zone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimestampValue {
    /// The date part.
    pub date: DateValue,
    /// The time part.
    pub time: TimeOfDay,
}
