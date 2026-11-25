//! Maps a `ClickHouse` physical type to a logical type (CAT-4.1) exactly as the E6-7.13 table
//! states. The type is read through `LowCardinality(…)` and then `Nullable(…)` to its inner
//! type, and the `Nullable` wrapper becomes the column's nullability (CAT-4). Every type the
//! table does not list is `unsupported` (CAT-4.3).

use menura_contracts::types::common::{FloatBits, IntegerBits, LogicalType, TimestampPrecision};

/// The largest decimal precision E6-7.13 maps; a wider `Decimal` is `unsupported`.
const MAXIMUM_DECIMAL_PRECISION: u8 = 38;

/// The logical type and nullability of a `ClickHouse` column type (E6-7.13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MappedType {
    /// The logical type of the inner type.
    pub(crate) logical_type: LogicalType,
    /// Whether the type is wrapped in `Nullable(…)`.
    pub(crate) nullable: bool,
}

/// The mapping of the `ClickHouse` type `physical_type`, as `system.columns` writes it
/// (E6-7.13).
pub(crate) fn map_clickhouse_type(physical_type: &str) -> MappedType {
    let name = physical_type.trim();
    let name = wrapped("LowCardinality", name).unwrap_or(name);
    match wrapped("Nullable", name) {
        Some(inner) => MappedType {
            logical_type: inner_logical_type(inner),
            nullable: true,
        },
        None => MappedType {
            logical_type: inner_logical_type(name),
            nullable: false,
        },
    }
}

/// The arguments of `name` when it is `<wrapper>(<arguments>)`.
fn wrapped<'name>(wrapper: &str, name: &'name str) -> Option<&'name str> {
    name.strip_prefix(wrapper)?
        .trim_start()
        .strip_prefix('(')?
        .strip_suffix(')')
        .map(str::trim)
}

/// The logical type of a type without the `LowCardinality` and `Nullable` wrappers of the
/// column itself.
fn inner_logical_type(name: &str) -> LogicalType {
    if let Some(element) = wrapped("Array", name) {
        return list_type(element);
    }
    let parameterized = [
        decimal_type as fn(&str) -> Option<LogicalType>,
        fixed_string_type,
        date_time_type,
        date_time64_type,
    ];
    parameterized
        .iter()
        .find_map(|parse| parse(name))
        .or_else(|| scalar_type(name))
        .unwrap_or(LogicalType::Unsupported)
}

/// `Array(T)` as a `list` whose element is the mapping of `T`, read through its own wrappers:
/// E6-7.2 writes `Nullable(<T>)` elements when they may be NULL, and a list has no element
/// nullability. A list of an `unsupported` element is itself `unsupported`.
fn list_type(element: &str) -> LogicalType {
    match map_clickhouse_type(element).logical_type {
        LogicalType::Unsupported => LogicalType::Unsupported,
        element => LogicalType::List {
            element: Box::new(element),
        },
    }
}

/// `Decimal(P, S)` with `P` from 1 to 38 and `S` from 0 to `P`.
fn decimal_type(name: &str) -> Option<LogicalType> {
    let (precision, scale) = wrapped("Decimal", name)?.split_once(',')?;
    let precision: u8 = precision.trim().parse().ok()?;
    let scale: u8 = scale.trim().parse().ok()?;
    let valid = (1..=MAXIMUM_DECIMAL_PRECISION).contains(&precision) && scale <= precision;
    valid.then_some(LogicalType::Decimal { precision, scale })
}

/// `FixedString(N)` with a positive `N` as `binary`.
fn fixed_string_type(name: &str) -> Option<LogicalType> {
    let length: u64 = wrapped("FixedString", name)?.parse().ok()?;
    (length > 0).then_some(LogicalType::Binary)
}

/// `DateTime('<zone>')` as an instant with precision `s`; without a zone it is `unsupported`.
fn date_time_type(name: &str) -> Option<LogicalType> {
    is_time_zone(wrapped("DateTime", name)?).then_some(instant(TimestampPrecision::S))
}

/// `DateTime64(P, '<zone>')` with `P` 0, 3, 6 or 9 as an instant of that precision; without a
/// zone it is `unsupported`.
fn date_time64_type(name: &str) -> Option<LogicalType> {
    let (digits, zone) = wrapped("DateTime64", name)?.split_once(',')?;
    if !is_time_zone(zone.trim()) {
        return None;
    }
    let precision = match digits.trim() {
        "0" => TimestampPrecision::S,
        "3" => TimestampPrecision::Ms,
        "6" => TimestampPrecision::Us,
        "9" => TimestampPrecision::Ns,
        _ => return None,
    };
    Some(instant(precision))
}

/// Whether `argument` is an explicit time-zone argument: a non-empty quoted name.
fn is_time_zone(argument: &str) -> bool {
    argument
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
        .is_some_and(|zone| !zone.is_empty() && !zone.contains('\''))
}

fn instant(precision: TimestampPrecision) -> LogicalType {
    LogicalType::Timestamp {
        precision,
        with_time_zone: true,
    }
}

/// The types of the E6-7.13 table that take no argument.
fn scalar_type(name: &str) -> Option<LogicalType> {
    let integer = |bits: IntegerBits, signed: bool| LogicalType::Integer { bits, signed };
    Some(match name {
        "Bool" => LogicalType::Boolean,
        "Int8" => integer(IntegerBits::Eight, true),
        "Int16" => integer(IntegerBits::Sixteen, true),
        "Int32" => integer(IntegerBits::ThirtyTwo, true),
        "Int64" => integer(IntegerBits::SixtyFour, true),
        "Int128" => integer(IntegerBits::OneHundredTwentyEight, true),
        "UInt8" => integer(IntegerBits::Eight, false),
        "UInt16" => integer(IntegerBits::Sixteen, false),
        "UInt32" => integer(IntegerBits::ThirtyTwo, false),
        "UInt64" => integer(IntegerBits::SixtyFour, false),
        "UInt128" => integer(IntegerBits::OneHundredTwentyEight, false),
        "Float32" => LogicalType::Float {
            bits: FloatBits::ThirtyTwo,
        },
        "Float64" => LogicalType::Float {
            bits: FloatBits::SixtyFour,
        },
        "String" => LogicalType::Text,
        "UUID" => LogicalType::Uuid,
        "Date" | "Date32" => LogicalType::Date,
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
