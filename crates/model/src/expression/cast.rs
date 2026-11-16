//! Target types of `CAST` (E0F-11).

/// The target type of a `CAST`. Synonyms share one variant: `VARCHAR` and `STRING`, `INTEGER`
/// and `INT`, `DECIMAL` and `NUMERIC`, `FLOAT` and `DOUBLE`, `TIMESTAMP` and `TIMESTAMP_NTZ`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CastTarget {
    /// `VARCHAR` or `STRING`: text.
    Text,
    /// `INTEGER` or `INT`: 32-bit signed integer.
    Integer,
    /// `BIGINT`: 64-bit signed integer.
    BigInteger,
    /// `DECIMAL(p, s)` or `NUMERIC(p, s)`.
    Decimal(DecimalTarget),
    /// `FLOAT` or `DOUBLE`: 64-bit float.
    Double,
    /// `BOOLEAN`.
    Boolean,
    /// `DATE`.
    Date,
    /// `TIME`.
    Time,
    /// `TIMESTAMP` or `TIMESTAMP_NTZ`: timestamp without time zone, precision `us`.
    Timestamp,
}

/// The precision and scale of a `DECIMAL` target: `1 ≤ precision ≤ 38`, `scale ≤ precision`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecimalTarget {
    precision: u8,
    scale: u8,
}

impl DecimalTarget {
    /// The largest precision (E0F-5a).
    pub const MAXIMUM_PRECISION: u8 = 38;

    /// Builds a decimal target. Returns `None` when the precision is outside 1–38 or the scale
    /// exceeds it.
    ///
    /// ```
    /// use semantic_compiler_model::expression::DecimalTarget;
    /// assert!(DecimalTarget::new(18, 2).is_some());
    /// assert!(DecimalTarget::new(2, 3).is_none());
    /// ```
    #[must_use]
    pub fn new(precision: u8, scale: u8) -> Option<Self> {
        let valid = (1..=Self::MAXIMUM_PRECISION).contains(&precision) && scale <= precision;
        valid.then_some(Self { precision, scale })
    }

    /// Total digits.
    #[must_use]
    pub fn precision(self) -> u8 {
        self.precision
    }

    /// Digits after the point.
    #[must_use]
    pub fn scale(self) -> u8 {
        self.scale
    }
}
