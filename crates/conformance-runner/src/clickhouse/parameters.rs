//! Binds the parameters of a compiled plan for `ClickHouse` (E6-7.3): the parameter at position
//! n is sent as the query parameter `pn`, its CON-4 value written in `ClickHouse`'s text form for
//! its logical type, and NULL written `\N`.

use menura_contracts::types::common::{LogicalType, Parameter};
use serde_json::Value as Json;

/// The `(name, text)` of every parameter of a plan, `p1`, `p2`, … in position order.
///
/// # Errors
///
/// Why a parameter cannot be bound: its positions are not 1, 2, …, its value does not have the
/// CON-4 encoding of its type, or `ClickHouse` has no text form for its type.
pub(crate) fn parameter_texts(parameters: &[Parameter]) -> Result<Vec<(String, String)>, String> {
    let mut ordered: Vec<&Parameter> = parameters.iter().collect();
    ordered.sort_by_key(|parameter| parameter.position);
    ordered
        .iter()
        .zip(1_u64..)
        .map(|(parameter, expected)| {
            if parameter.position != expected {
                return Err(format!("no parameter has the position {expected}"));
            }
            field_text(&parameter.logical_type, &parameter.value)
                .map(|text| (format!("p{expected}"), text))
                .map_err(|reason| format!("parameter p{expected}: {reason}"))
        })
        .collect()
}

/// The text of a whole parameter, in the escaped form of a `TabSeparated` field.
fn field_text(logical_type: &LogicalType, value: &Json) -> Result<String, String> {
    if value.is_null() {
        return Ok("\\N".to_owned());
    }
    match logical_type {
        LogicalType::List { element } => list_text(element, value),
        LogicalType::Text | LogicalType::Uuid => Ok(escape(&scalar_text(logical_type, value)?)),
        _ => scalar_text(logical_type, value),
    }
}

/// The text of a non-NULL scalar value, before any escaping.
fn scalar_text(logical_type: &LogicalType, value: &Json) -> Result<String, String> {
    let invalid = || format!("`{value}` is not a CON-4 value of type {logical_type:?}");
    match logical_type {
        LogicalType::Boolean => value
            .as_bool()
            .map(|flag| flag.to_string())
            .ok_or_else(invalid),
        LogicalType::Integer { .. } => match value {
            Json::Number(number) if number.is_i64() || number.is_u64() => Ok(number.to_string()),
            Json::String(text) if text.parse::<i128>().is_ok() => Ok(text.clone()),
            _ => Err(invalid()),
        },
        LogicalType::Float { .. } => value
            .as_f64()
            .map(|number| number.to_string())
            .ok_or_else(invalid),
        LogicalType::Decimal { .. } | LogicalType::Text | LogicalType::Date | LogicalType::Uuid => {
            value.as_str().map(str::to_owned).ok_or_else(invalid)
        }
        LogicalType::Timestamp {
            with_time_zone: true,
            ..
        } => value.as_str().and_then(instant_text).ok_or_else(invalid),
        _ => Err(format!(
            "the runner cannot bind a parameter of type {logical_type:?} on ClickHouse"
        )),
    }
}

/// The `ClickHouse` text of a CON-4 instant `YYYY-MM-DDTHH:MM:SS[.f]Z`, read in the `UTC` time
/// zone of its `DateTime64` type (E6-7.2): `YYYY-MM-DD HH:MM:SS[.f]`.
fn instant_text(text: &str) -> Option<String> {
    let utc = text.strip_suffix('Z')?;
    let (date, time) = utc.split_once('T')?;
    Some(format!("{date} {time}"))
}

/// The text of a list: `[e1,e2]`, each element in its quoted form.
fn list_text(element: &LogicalType, value: &Json) -> Result<String, String> {
    let items = value
        .as_array()
        .ok_or_else(|| format!("`{value}` is not a CON-4 list"))?;
    let texts = items
        .iter()
        .map(|item| element_text(element, item))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(format!("[{}]", texts.join(",")))
}

/// The quoted form of one list element: `NULL`, a number or a boolean as it is, any other value
/// between single quotes.
fn element_text(element: &LogicalType, item: &Json) -> Result<String, String> {
    if item.is_null() {
        return Ok("NULL".to_owned());
    }
    let text = scalar_text(element, item)?;
    Ok(match element {
        LogicalType::Boolean | LogicalType::Integer { .. } | LogicalType::Float { .. } => text,
        _ => format!("'{}'", escape(&text).replace('\'', "\\'")),
    })
}

/// `text` with the escapes of `ClickHouse`'s escaped text form: backslash, tab, line feed and
/// carriage return.
pub(crate) fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::common::{IntegerBits, TimestampPrecision};
    use serde_json::json;

    use super::*;

    fn parameter(position: u64, logical_type: LogicalType, value: Json) -> Parameter {
        Parameter {
            position,
            logical_type,
            value,
        }
    }

    fn integer() -> LogicalType {
        LogicalType::Integer {
            bits: IntegerBits::SixtyFour,
            signed: true,
        }
    }

    #[test]
    fn e6_7_3_binds_position_n_to_the_parameter_pn() {
        let parameters = [
            parameter(2, LogicalType::Text, json!("a\tb")),
            parameter(1, integer(), json!(7)),
        ];
        assert_eq!(
            parameter_texts(&parameters).unwrap(),
            vec![
                ("p1".to_owned(), "7".to_owned()),
                ("p2".to_owned(), "a\\tb".to_owned())
            ]
        );
    }

    #[test]
    fn e6_7_3_writes_null_as_backslash_n() {
        let parameters = [parameter(1, LogicalType::Date, Json::Null)];
        assert_eq!(parameter_texts(&parameters).unwrap()[0].1, "\\N");
    }

    #[test]
    fn e6_7_3_writes_an_instant_in_utc_without_the_z() {
        let instant = LogicalType::Timestamp {
            precision: TimestampPrecision::Us,
            with_time_zone: true,
        };
        let parameters = [parameter(1, instant, json!("2026-01-05T10:00:00.5Z"))];
        assert_eq!(
            parameter_texts(&parameters).unwrap()[0].1,
            "2026-01-05 10:00:00.5"
        );
    }

    #[test]
    fn e6_7_3_writes_a_list_with_quoted_elements_and_null() {
        let list = LogicalType::List {
            element: Box::new(LogicalType::Text),
        };
        let parameters = [parameter(1, list, json!(["it's", null, "a\\b"]))];
        assert_eq!(
            parameter_texts(&parameters).unwrap()[0].1,
            "['it\\'s',NULL,'a\\\\b']"
        );
    }

    #[test]
    fn e6_7_3_refuses_a_gap_in_the_positions() {
        let parameters = [parameter(2, integer(), json!(1))];
        assert!(parameter_texts(&parameters).is_err());
    }

    #[test]
    fn e6_7_3_keeps_a_large_integer_written_as_a_string() {
        let parameters = [parameter(1, integer(), json!("9007199254740993"))];
        assert_eq!(
            parameter_texts(&parameters).unwrap()[0].1,
            "9007199254740993"
        );
    }
}
