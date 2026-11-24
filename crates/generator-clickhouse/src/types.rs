//! `ClickHouse` type names (E6-7.2) and the logical types of attribute parameters (E6-5.7).

use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};
use menura_model::internal_model::{AttributeType, ScalarType};

/// The `ClickHouse` name of `logical_type` (E6-7.2), or `None` for a type the dialect cannot
/// express (`time`, `interval`, `timestamp` without time zone, E6-7.12), `unsupported`, and a
/// list of one of them.
pub(crate) fn physical_type(logical_type: &LogicalType) -> Option<String> {
    let name = match logical_type {
        LogicalType::Boolean => "Bool",
        LogicalType::Integer { bits, signed } => integer_name(*bits, *signed),
        LogicalType::Decimal { precision, scale } => {
            return Some(format!("Decimal({precision}, {scale})"));
        }
        LogicalType::Float {
            bits: FloatBits::ThirtyTwo,
        } => "Float32",
        LogicalType::Float {
            bits: FloatBits::SixtyFour,
        } => "Float64",
        LogicalType::Text | LogicalType::Binary => "String",
        LogicalType::Uuid => "UUID",
        LogicalType::Date => "Date32",
        LogicalType::Timestamp {
            precision,
            with_time_zone: true,
        } => return Some(format!("DateTime64({}, 'UTC')", digits(*precision))),
        LogicalType::List { element } => {
            return physical_type(element).map(|name| format!("Array({name})"));
        }
        LogicalType::Time
        | LogicalType::Interval
        | LogicalType::Timestamp {
            with_time_zone: false,
            ..
        }
        | LogicalType::Unsupported => return None,
    };
    Some(name.to_owned())
}

/// The name of a value of `logical_type` that may be NULL: `Nullable(<name>)`, except for a
/// list, which `ClickHouse` does not allow inside `Nullable` (E6-7.2).
pub(crate) fn nullable_type(logical_type: &LogicalType) -> Option<String> {
    let name = physical_type(logical_type)?;
    Some(match logical_type {
        LogicalType::List { .. } => name,
        _ => format!("Nullable({name})"),
    })
}

/// `value` cast to `logical_type`, NULL kept (E6-7.9): `accurateCast(value, 'Nullable(<T>)')`
/// for an integer, which raises an error when the value does not fit where `CAST` would wrap
/// around, and `CAST(value AS Nullable(<T>))` otherwise. `None` when the type has no name.
pub(crate) fn cast_sql(value: &str, logical_type: &LogicalType) -> Option<String> {
    let name = nullable_type(logical_type)?;
    Some(match logical_type {
        LogicalType::Integer { .. } => format!("accurateCast({value}, '{name}')"),
        _ => format!("CAST({value} AS {name})"),
    })
}

/// The name of a column of `logical_type`: [`nullable_type`] when it is `nullable`, the plain
/// name otherwise (E6-7.2).
pub(crate) fn column_type(logical_type: &LogicalType, nullable: bool) -> Option<String> {
    if nullable {
        nullable_type(logical_type)
    } else {
        physical_type(logical_type)
    }
}

/// Whether `logical_type` is a type `ClickHouse` cannot express (E6-7.12): `time`, `interval`,
/// `timestamp` without time zone, or a list of one of them.
pub(crate) fn is_inexpressible(logical_type: &LogicalType) -> bool {
    match logical_type {
        LogicalType::Time
        | LogicalType::Interval
        | LogicalType::Timestamp {
            with_time_zone: false,
            ..
        } => true,
        LogicalType::List { element } => is_inexpressible(element),
        _ => false,
    }
}

/// Whether a value of `logical_type`, NULL when `nullable`, has no `ClickHouse` type
/// (E6-7.12): a type of [`is_inexpressible`], or a nullable list, since `ClickHouse` has no
/// `Nullable(Array(…))`.
pub(crate) fn is_inexpressible_value(logical_type: &LogicalType, nullable: bool) -> bool {
    is_inexpressible(logical_type) || (nullable && matches!(logical_type, LogicalType::List { .. }))
}

/// The number of fraction digits of a timestamp of `precision`: 0, 3, 6 or 9 (E6-7.2).
pub(crate) fn digits(precision: TimestampPrecision) -> u8 {
    match precision {
        TimestampPrecision::S => 0,
        TimestampPrecision::Ms => 3,
        TimestampPrecision::Us => 6,
        TimestampPrecision::Ns => 9,
    }
}

fn integer_name(bits: IntegerBits, signed: bool) -> &'static str {
    match (bits, signed) {
        (IntegerBits::Eight, true) => "Int8",
        (IntegerBits::Sixteen, true) => "Int16",
        (IntegerBits::ThirtyTwo, true) => "Int32",
        (IntegerBits::SixtyFour, true) => "Int64",
        (IntegerBits::OneHundredTwentyEight, true) => "Int128",
        (IntegerBits::Eight, false) => "UInt8",
        (IntegerBits::Sixteen, false) => "UInt16",
        (IntegerBits::ThirtyTwo, false) => "UInt32",
        (IntegerBits::SixtyFour, false) => "UInt64",
        (IntegerBits::OneHundredTwentyEight, false) => "UInt128",
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

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::{IntegerBits, LogicalType, TimestampPrecision};

    use super::{nullable_type, physical_type};

    fn name(logical_type: &LogicalType) -> Option<String> {
        physical_type(logical_type)
    }

    #[test]
    fn e6_7_2_logical_types_have_clickhouse_names() {
        let unsigned = LogicalType::Integer {
            bits: IntegerBits::ThirtyTwo,
            signed: false,
        };
        let timestamp = LogicalType::Timestamp {
            precision: TimestampPrecision::Ms,
            with_time_zone: true,
        };
        let list = LogicalType::List {
            element: Box::new(LogicalType::Text),
        };
        assert_eq!(name(&LogicalType::Boolean).as_deref(), Some("Bool"));
        assert_eq!(name(&unsigned).as_deref(), Some("UInt32"));
        let decimal = LogicalType::Decimal {
            precision: 12,
            scale: 2,
        };
        assert_eq!(name(&decimal).as_deref(), Some("Decimal(12, 2)"));
        assert_eq!(name(&LogicalType::Binary).as_deref(), Some("String"));
        assert_eq!(name(&LogicalType::Date).as_deref(), Some("Date32"));
        assert_eq!(name(&timestamp).as_deref(), Some("DateTime64(3, 'UTC')"));
        assert_eq!(name(&list).as_deref(), Some("Array(String)"));
        assert_eq!(nullable_type(&list).as_deref(), Some("Array(String)"));
        assert_eq!(
            nullable_type(&LogicalType::Uuid).as_deref(),
            Some("Nullable(UUID)")
        );
    }

    #[test]
    fn e6_7_12_time_interval_and_naive_timestamps_have_no_name() {
        let naive = LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: false,
        };
        assert_eq!(name(&LogicalType::Time), None);
        assert_eq!(name(&LogicalType::Interval), None);
        assert_eq!(name(&naive), None);
    }
}
