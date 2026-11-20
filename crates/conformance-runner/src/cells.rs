//! The values CS-2.4 compares: a value the plan returns or a field of the `rows` file, reduced
//! by the logical type of its column to NULL, an exact number, a float or its CON-4 text.

use duckdb::types::Value;
use menura_contracts::types::common::LogicalType;

use crate::calendar::{format_date, format_time, format_timestamp};

/// A value reduced to what CS-2.4 compares.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Cell {
    Null,
    /// An integer or a decimal: the digits without trailing fraction zeros, and the scale left.
    Exact {
        mantissa: i128,
        scale: u32,
    },
    Float(f64),
    Text(String),
}

/// The cell of a value the plan returned in a column of type `logical_type`.
pub(crate) fn actual_cell(value: &Value, logical_type: &LogicalType) -> Result<Cell, String> {
    let exact = |mantissa: i128| Ok(exact_cell(mantissa, 0));
    match value {
        Value::Null => Ok(Cell::Null),
        Value::Boolean(boolean) => Ok(Cell::Text(boolean.to_string())),
        Value::TinyInt(integer) => exact(i128::from(*integer)),
        Value::SmallInt(integer) => exact(i128::from(*integer)),
        Value::Int(integer) => exact(i128::from(*integer)),
        Value::BigInt(integer) => exact(i128::from(*integer)),
        Value::HugeInt(integer) => exact(*integer),
        Value::UTinyInt(integer) => exact(i128::from(*integer)),
        Value::USmallInt(integer) => exact(i128::from(*integer)),
        Value::UInt(integer) => exact(i128::from(*integer)),
        Value::UBigInt(integer) => exact(i128::from(*integer)),
        Value::UHugeInt(integer) => i128::try_from(*integer)
            .map_err(|_| format!("the value {integer} is out of the compared range"))
            .and_then(exact),
        Value::Decimal(decimal) => Ok(exact_cell(decimal.value(), u32::from(decimal.scale()))),
        Value::Float(float) => Ok(Cell::Float(f64::from(*float))),
        Value::Double(float) => Ok(Cell::Float(*float)),
        Value::Text(text) | Value::Enum(text) => Ok(Cell::Text(text.clone())),
        Value::Date32(days) => Ok(Cell::Text(format_date(i64::from(*days)))),
        Value::Time64(unit, time) => Ok(Cell::Text(format_time(unit.to_micros(*time)))),
        Value::Timestamp(unit, timestamp) => {
            let with_time_zone = matches!(
                logical_type,
                LogicalType::Timestamp {
                    with_time_zone: true,
                    ..
                }
            );
            Ok(Cell::Text(format_timestamp(
                unit.to_micros(*timestamp),
                with_time_zone,
            )))
        }
        other => Err(format!("the runner cannot compare the value {other:?}")),
    }
}

/// The cell of an expected field in a column of type `logical_type`.
pub(crate) fn expected_cell(
    field: Option<&str>,
    logical_type: &LogicalType,
) -> Result<Cell, String> {
    let Some(text) = field else {
        return Ok(Cell::Null);
    };
    match logical_type {
        LogicalType::Integer { .. } | LogicalType::Decimal { .. } => {
            exact_of_text(text).ok_or_else(|| format!("`{text}` is not an exact number"))
        }
        LogicalType::Float { .. } => text
            .parse()
            .map(Cell::Float)
            .map_err(|_| format!("`{text}` is not a float")),
        _ => Ok(Cell::Text(text.to_owned())),
    }
}

/// An exact number written with an optional `-`, digits and an optional fraction.
fn exact_of_text(text: &str) -> Option<Cell> {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if whole.is_empty() || !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    let magnitude: i128 = format!("{whole}{fraction}").parse().ok()?;
    let mantissa = if text.starts_with('-') {
        -magnitude
    } else {
        magnitude
    };
    Some(exact_cell(mantissa, u32::try_from(fraction.len()).ok()?))
}

/// An exact number without trailing fraction zeros, so that `45` and `45.00` are equal.
fn exact_cell(mut mantissa: i128, mut scale: u32) -> Cell {
    while scale > 0 && mantissa % 10 == 0 {
        mantissa /= 10;
        scale -= 1;
    }
    Cell::Exact { mantissa, scale }
}

/// The rows as text, for failure reasons.
pub(crate) fn describe_rows(rows: &[Vec<Cell>]) -> String {
    let row = |cells: &Vec<Cell>| {
        let values: Vec<String> = cells.iter().map(describe_cell).collect();
        format!("({})", values.join(", "))
    };
    let rows: Vec<String> = rows.iter().map(row).collect();
    format!("[{}]", rows.join(", "))
}

fn describe_cell(cell: &Cell) -> String {
    match cell {
        Cell::Null => "NULL".to_owned(),
        Cell::Exact { mantissa, scale } => describe_exact(*mantissa, *scale),
        Cell::Float(float) => format!("{float:?}"),
        Cell::Text(text) => format!("{text:?}"),
    }
}

fn describe_exact(mantissa: i128, scale: u32) -> String {
    let digits = mantissa.unsigned_abs().to_string();
    let sign = if mantissa < 0 { "-" } else { "" };
    let Ok(scale) = usize::try_from(scale) else {
        return format!("{sign}{digits}e-{scale}");
    };
    if scale == 0 {
        return format!("{sign}{digits}");
    }
    let padded = format!("{digits:0>width$}", width = scale + 1);
    let (whole, fraction) = padded.split_at(padded.len() - scale);
    format!("{sign}{whole}.{fraction}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_numbers_are_described_with_their_scale() {
        assert_eq!(describe_exact(-5, 2), "-0.05");
        assert_eq!(describe_exact(3075, 2), "30.75");
        assert_eq!(describe_exact(45, 0), "45");
    }

    #[test]
    fn cs_2_4_exact_numbers_ignore_trailing_fraction_zeros() {
        assert_eq!(exact_of_text("45.00"), exact_of_text("45"));
        assert_eq!(exact_of_text("-0.50"), Some(exact_cell(-5, 1)));
        assert_eq!(exact_of_text("1e3"), None);
        assert_eq!(exact_of_text(".5"), None);
    }
}
