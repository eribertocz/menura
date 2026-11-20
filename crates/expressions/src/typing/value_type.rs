//! The type of an expression: a logical type of the catalog schema (CAT-4.1) or the type of
//! the literal `NULL`, with whether the expression can be NULL (E0F-7.1).

use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};

/// The type of an expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionType {
    /// The type of the literal `NULL`, which converts implicitly to every type.
    Null,
    /// A value of a logical type; never [`LogicalType::Unsupported`], whose use is `E0F-04`.
    Value(LogicalType),
}

/// The type of an expression and whether it can be NULL (E0F-7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Typed {
    /// The result type.
    pub value_type: ExpressionType,
    /// Whether the expression can be NULL, derived from `nullable` in the catalog schema.
    pub nullable: bool,
}

impl Typed {
    /// An expression of the logical type `value_type`.
    #[must_use]
    pub fn value(value_type: LogicalType, nullable: bool) -> Self {
        Self {
            value_type: ExpressionType::Value(value_type),
            nullable,
        }
    }

    /// The logical type, unless the expression is the literal `NULL`.
    #[must_use]
    pub fn logical(&self) -> Option<&LogicalType> {
        match &self.value_type {
            ExpressionType::Null => None,
            ExpressionType::Value(value_type) => Some(value_type),
        }
    }
}

/// A numeric logical type, by kind (E0F-10: *integer*, *decimal*, *float*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Numeric {
    /// An integer of a fixed width.
    Integer {
        /// Width in bits.
        bits: u16,
        /// Whether negative values exist.
        signed: bool,
    },
    /// `decimal(precision, scale)`.
    Decimal {
        /// Total digits.
        precision: u8,
        /// Digits after the point.
        scale: u8,
    },
    /// A binary float.
    Float,
}

/// The numeric kind of `value_type`, or `None` when it is not numeric.
pub(crate) fn numeric(value_type: &LogicalType) -> Option<Numeric> {
    match value_type {
        LogicalType::Integer { bits, signed } => Some(Numeric::Integer {
            bits: u16::from(*bits),
            signed: *signed,
        }),
        LogicalType::Decimal { precision, scale } => Some(Numeric::Decimal {
            precision: *precision,
            scale: *scale,
        }),
        LogicalType::Float { .. } => Some(Numeric::Float),
        _ => None,
    }
}

/// Whether `value_type` is an integer of any width.
pub(crate) fn is_integer(value_type: &LogicalType) -> bool {
    matches!(value_type, LogicalType::Integer { .. })
}

/// The 32-bit signed integer.
#[must_use]
pub fn integer() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::ThirtyTwo,
        signed: true,
    }
}

/// The 64-bit signed integer.
#[must_use]
pub fn big_integer() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::SixtyFour,
        signed: true,
    }
}

/// The 128-bit signed integer.
#[must_use]
pub fn huge_integer() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::OneHundredTwentyEight,
        signed: true,
    }
}

/// The 64-bit float.
#[must_use]
pub fn double() -> LogicalType {
    LogicalType::Float {
        bits: FloatBits::SixtyFour,
    }
}

/// `decimal(precision, scale)`.
#[must_use]
pub fn decimal(precision: u8, scale: u8) -> LogicalType {
    LogicalType::Decimal { precision, scale }
}

/// The timestamp without time zone, precision `us`, of literals, `CAST` and `CURRENT_TIMESTAMP`
/// (E0F-9.3, E0F-11).
#[must_use]
pub fn timestamp() -> LogicalType {
    LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone: false,
    }
}

/// The name of a type, for messages.
#[must_use]
pub fn describe(value_type: &ExpressionType) -> String {
    let ExpressionType::Value(value_type) = value_type else {
        return "NULL".to_owned();
    };
    match value_type {
        LogicalType::Boolean => "boolean".to_owned(),
        LogicalType::Integer { bits, signed } => {
            let sign = if *signed { "signed" } else { "unsigned" };
            format!("{}-bit {sign} integer", u16::from(*bits))
        }
        LogicalType::Decimal { precision, scale } => format!("decimal({precision},{scale})"),
        LogicalType::Float { bits } => format!("{}-bit float", u16::from(*bits)),
        LogicalType::Text => "text".to_owned(),
        LogicalType::Date => "date".to_owned(),
        LogicalType::Time => "time".to_owned(),
        LogicalType::Timestamp { with_time_zone, .. } if *with_time_zone => {
            "timestamp with time zone".to_owned()
        }
        LogicalType::Timestamp { .. } => "timestamp".to_owned(),
        LogicalType::Interval => "interval".to_owned(),
        LogicalType::Binary => "binary".to_owned(),
        LogicalType::Uuid => "uuid".to_owned(),
        LogicalType::List { .. } => "list".to_owned(),
        LogicalType::Unsupported => "unsupported".to_owned(),
    }
}
