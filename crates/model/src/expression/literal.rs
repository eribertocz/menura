//! Literal values of the accepted subset (E0F-2.2, E0F-9.3).

/// A constant written in an expression, of one of the literal kinds of E0F-9.3.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Literal {
    /// `NULL`.
    Null,
    /// `TRUE` or `FALSE`.
    Boolean(bool),
    /// A text literal, with `''` already undoubled.
    Text(String),
    /// A number with digits only, such as `12`.
    Integer(IntegerLiteral),
    /// A number with `.` and no exponent, such as `0.50`.
    Decimal(DecimalLiteral),
    /// A number with an exponent, such as `1.5E3`: a 64-bit float.
    Float(FloatLiteral),
    /// `DATE 'YYYY-MM-DD'`.
    Date(DateValue),
    /// `TIME 'HH:MM:SS[.fraction]'`.
    Time(TimeOfDay),
    /// `TIMESTAMP '…'` or `TIMESTAMP_NTZ '…'`: both are a timestamp without time zone.
    Timestamp(TimestampValue),
}

/// The most digits an exact literal may have (E0F-9.3).
pub const MAXIMUM_EXACT_DIGITS: usize = 38;

fn all_digits(text: &str) -> bool {
    text.bytes().all(|byte| byte.is_ascii_digit())
}

/// An integer literal: its digits exactly as written, at most 38 (E0F-9.3).
///
/// The digits are kept as written, leading zeros included, because the literal's type depends
/// on how many there are: up to 18 a 64-bit integer, 19 to 38 `decimal(n,0)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntegerLiteral {
    digits: String,
}

impl IntegerLiteral {
    /// The most digits of a literal typed as a 64-bit integer (E0F-9.3).
    pub const MAXIMUM_BIG_INTEGER_DIGITS: usize = 18;

    /// Builds an integer literal. Returns `None` when `digits` is empty, holds anything but
    /// ASCII digits, or has more than 38 digits.
    ///
    /// ```
    /// use menura_model::expression::IntegerLiteral;
    /// assert!(IntegerLiteral::new("007").is_some());
    /// assert!(IntegerLiteral::new(&"9".repeat(39)).is_none());
    /// ```
    #[must_use]
    pub fn new(digits: &str) -> Option<Self> {
        let valid =
            !digits.is_empty() && digits.len() <= MAXIMUM_EXACT_DIGITS && all_digits(digits);
        valid.then(|| Self {
            digits: digits.to_owned(),
        })
    }

    /// The digits as written.
    #[must_use]
    pub fn digits(&self) -> &str {
        &self.digits
    }
}

/// A decimal literal: the digits before and after the point exactly as written, at most 38
/// in all (E0F-9.3). Its type is `decimal(p,s)` with `p` all the digits and `s` the fraction
/// digits: `.5` is `decimal(1,1)`, `10.` is `decimal(2,0)`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DecimalLiteral {
    integer_digits: String,
    fraction_digits: String,
}

impl DecimalLiteral {
    /// Builds a decimal literal. Returns `None` when a part holds anything but ASCII digits,
    /// when both parts are empty, or when they have more than 38 digits together.
    ///
    /// ```
    /// use menura_model::expression::DecimalLiteral;
    /// let number = DecimalLiteral::new("", "5").unwrap();
    /// assert_eq!((number.precision(), number.scale()), (1, 1));
    /// ```
    #[must_use]
    pub fn new(integer_digits: &str, fraction_digits: &str) -> Option<Self> {
        let length = integer_digits.len() + fraction_digits.len();
        let valid = length > 0
            && length <= MAXIMUM_EXACT_DIGITS
            && all_digits(integer_digits)
            && all_digits(fraction_digits);
        valid.then(|| Self {
            integer_digits: integer_digits.to_owned(),
            fraction_digits: fraction_digits.to_owned(),
        })
    }

    /// The digits before the point, as written; may be empty (`.5`).
    #[must_use]
    pub fn integer_digits(&self) -> &str {
        &self.integer_digits
    }

    /// The digits after the point, as written; may be empty (`10.`).
    #[must_use]
    pub fn fraction_digits(&self) -> &str {
        &self.fraction_digits
    }

    /// Every digit written, 1 to 38.
    #[must_use]
    pub fn precision(&self) -> usize {
        self.integer_digits.len() + self.fraction_digits.len()
    }

    /// The digits after the point.
    #[must_use]
    pub fn scale(&self) -> usize {
        self.fraction_digits.len()
    }
}

/// A float literal: `mantissa × 10^exponent`, with the mantissa digits as written.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FloatLiteral {
    integer_digits: String,
    fraction_digits: String,
    exponent: i32,
}

impl FloatLiteral {
    /// Builds a float literal. Returns `None` when a mantissa part holds anything but ASCII
    /// digits or when both are empty.
    #[must_use]
    pub fn new(integer_digits: &str, fraction_digits: &str, exponent: i32) -> Option<Self> {
        let valid = !(integer_digits.is_empty() && fraction_digits.is_empty())
            && all_digits(integer_digits)
            && all_digits(fraction_digits);
        valid.then(|| Self {
            integer_digits: integer_digits.to_owned(),
            fraction_digits: fraction_digits.to_owned(),
            exponent,
        })
    }

    /// The mantissa digits before the point, as written.
    #[must_use]
    pub fn integer_digits(&self) -> &str {
        &self.integer_digits
    }

    /// The mantissa digits after the point, as written; empty when there is no point.
    #[must_use]
    pub fn fraction_digits(&self) -> &str {
        &self.fraction_digits
    }

    /// The power of ten.
    #[must_use]
    pub fn exponent(&self) -> i32 {
        self.exponent
    }
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
    /// use menura_model::expression::DateValue;
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

/// A time of day with up to six fraction digits of a second (E0F-9.3).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimeOfDay {
    hour: u8,
    minute: u8,
    second: u8,
    fraction_digits: String,
}

impl TimeOfDay {
    /// The most fraction digits of a second (E0F-9.3).
    pub const MAXIMUM_FRACTION_DIGITS: usize = 6;

    /// Builds a time of day. Returns `None` when a field is out of range, or when the fraction
    /// has more than six digits or anything but ASCII digits.
    #[must_use]
    pub fn new(hour: u8, minute: u8, second: u8, fraction_digits: &str) -> Option<Self> {
        let valid = hour < 24
            && minute < 60
            && second < 60
            && fraction_digits.len() <= Self::MAXIMUM_FRACTION_DIGITS
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

    /// The fraction digits of the second as written, at most six; empty when none.
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
