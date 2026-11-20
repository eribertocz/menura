//! `DuckDB` type names (E6-5.6) and the logical types of attribute parameters (E6-5.7).

use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};
use menura_model::internal_model::{AttributeType, ScalarType};

/// The `DuckDB` name of `logical_type` in the CAT-4.4 table (E6-5.6), or `None` for
/// `unsupported` and a list of it.
pub(crate) fn physical_type(logical_type: &LogicalType) -> Option<String> {
    let name = match logical_type {
        LogicalType::Boolean => "BOOLEAN",
        LogicalType::Integer { bits, signed } => integer_name(*bits, *signed),
        LogicalType::Decimal { precision, scale } => {
            return Some(format!("DECIMAL({precision},{scale})"));
        }
        LogicalType::Float {
            bits: FloatBits::ThirtyTwo,
        } => "FLOAT",
        LogicalType::Float {
            bits: FloatBits::SixtyFour,
        } => "DOUBLE",
        LogicalType::Text => "VARCHAR",
        LogicalType::Date => "DATE",
        LogicalType::Time => "TIME",
        LogicalType::Timestamp {
            precision,
            with_time_zone,
        } => timestamp_name(*precision, *with_time_zone),
        LogicalType::Interval => "INTERVAL",
        LogicalType::Binary => "BLOB",
        LogicalType::Uuid => "UUID",
        LogicalType::List { element } => return physical_type(element).map(|name| name + "[]"),
        LogicalType::Unsupported => return None,
    };
    Some(name.to_owned())
}

fn integer_name(bits: IntegerBits, signed: bool) -> &'static str {
    match (bits, signed) {
        (IntegerBits::Eight, true) => "TINYINT",
        (IntegerBits::Sixteen, true) => "SMALLINT",
        (IntegerBits::ThirtyTwo, true) => "INTEGER",
        (IntegerBits::SixtyFour, true) => "BIGINT",
        (IntegerBits::OneHundredTwentyEight, true) => "HUGEINT",
        (IntegerBits::Eight, false) => "UTINYINT",
        (IntegerBits::Sixteen, false) => "USMALLINT",
        (IntegerBits::ThirtyTwo, false) => "UINTEGER",
        (IntegerBits::SixtyFour, false) => "UBIGINT",
        (IntegerBits::OneHundredTwentyEight, false) => "UHUGEINT",
    }
}

/// Every timestamp with time zone has precision `us` in `DuckDB` (CAT-4.4).
fn timestamp_name(precision: TimestampPrecision, with_time_zone: bool) -> &'static str {
    if with_time_zone {
        return "TIMESTAMP WITH TIME ZONE";
    }
    match precision {
        TimestampPrecision::S => "TIMESTAMP_S",
        TimestampPrecision::Ms => "TIMESTAMP_MS",
        TimestampPrecision::Us => "TIMESTAMP",
        TimestampPrecision::Ns => "TIMESTAMP_NS",
    }
}

/// The logical type of a parameter bound from an attribute declared as `attribute_type`
/// (E6-5.7).
pub(crate) fn attribute_logical_type(attribute_type: AttributeType) -> LogicalType {
    match attribute_type {
        AttributeType::Scalar(scalar) => scalar_logical_type(scalar),
        AttributeType::List(scalar) => LogicalType::List {
            element: Box::new(scalar_logical_type(scalar)),
        },
    }
}

fn scalar_logical_type(scalar: ScalarType) -> LogicalType {
    match scalar {
        ScalarType::Text => LogicalType::Text,
        ScalarType::Integer => LogicalType::Integer {
            bits: IntegerBits::SixtyFour,
            signed: true,
        },
        ScalarType::Decimal { precision, scale } => LogicalType::Decimal { precision, scale },
        ScalarType::Date => LogicalType::Date,
        ScalarType::Boolean => LogicalType::Boolean,
    }
}
