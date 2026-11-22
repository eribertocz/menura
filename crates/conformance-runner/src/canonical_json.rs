//! JSON values compared as CS-2.10 states: parsed without duplicate member names, and written
//! as RFC 8785 canonical JSON or in the stored form of a golden file.

use std::cmp::Ordering;
use std::fmt;
use std::fmt::Write as _;

use serde::Serialize;
use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// A JSON value whose object members keep their names in RFC 8785 order and whose numbers are
/// IEEE 754 doubles, as RFC 8785 reads them.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JsonValue {
    /// `null`.
    Null,
    /// `true` or `false`.
    Bool(bool),
    /// A number, read as a double (RFC 8785 section 3.2.2.3).
    Number(f64),
    /// A string.
    String(String),
    /// An array, in its order.
    Array(Vec<JsonValue>),
    /// An object, its members sorted by the UTF-16 code units of their names (RFC 8785
    /// section 3.2.3).
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// Parses JSON text, failing on invalid JSON and on a duplicate member name (CS-2.10).
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| error.to_string())
    }

    /// The JSON value of a serializable output, such as a member of a compiled plan.
    pub(crate) fn of(value: &impl Serialize) -> Result<Self, String> {
        let value = serde_json::to_value(value).map_err(|error| error.to_string())?;
        Self::deserialize(value).map_err(|error| error.to_string())
    }

    /// The member `name` of an object, or `None`.
    pub(crate) fn member(&self, name: &str) -> Option<&Self> {
        match self {
            Self::Object(members) => members
                .iter()
                .find(|(member, _)| member == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// The value without the members `names`, when it is an object.
    pub(crate) fn without(&self, names: &[&str]) -> Self {
        match self {
            Self::Object(members) => Self::Object(
                members
                    .iter()
                    .filter(|(member, _)| !names.contains(&member.as_str()))
                    .cloned()
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// The RFC 8785 canonical JSON text of the value.
    pub(crate) fn canonical(&self) -> String {
        let mut text = String::new();
        self.write(&mut text, None);
        text
    }

    /// The stored form of a golden JSON file (CS-2.10): canonical members, numbers and strings,
    /// two spaces of indentation per level, one member or element per line, and one line feed
    /// at the end.
    pub(crate) fn stored_form(&self) -> String {
        let mut text = String::new();
        self.write(&mut text, Some(0));
        text.push('\n');
        text
    }

    /// Writes the value; `indent` is the nesting level of the stored form, or `None` for
    /// canonical JSON.
    fn write(&self, text: &mut String, indent: Option<usize>) {
        match self {
            Self::Null => text.push_str("null"),
            Self::Bool(value) => text.push_str(if *value { "true" } else { "false" }),
            Self::Number(value) => text.push_str(&number_text(*value)),
            Self::String(value) => write_string(text, value),
            Self::Array(elements) => {
                write_container(
                    text,
                    indent,
                    ('[', ']'),
                    elements,
                    |text, element, inner| {
                        element.write(text, inner);
                    },
                );
            }
            Self::Object(members) => {
                write_container(text, indent, ('{', '}'), members, |text, member, inner| {
                    write_string(text, &member.0);
                    text.push_str(if inner.is_some() { ": " } else { ":" });
                    member.1.write(text, inner);
                });
            }
        }
    }
}

/// Writes an array or an object: `[]` or `{}` when empty, otherwise one item per line in the
/// stored form, or no white space in canonical JSON.
fn write_container<Item>(
    text: &mut String,
    indent: Option<usize>,
    (open, close): (char, char),
    items: &[Item],
    write_item: impl Fn(&mut String, &Item, Option<usize>),
) {
    text.push(open);
    let inner = indent.map(|level| level + 1);
    for (position, item) in items.iter().enumerate() {
        if position > 0 {
            text.push(',');
        }
        if let Some(level) = inner {
            new_line(text, level);
        }
        write_item(text, item, inner);
    }
    if let (Some(level), false) = (indent, items.is_empty()) {
        new_line(text, level);
    }
    text.push(close);
}

fn new_line(text: &mut String, level: usize) {
    text.push('\n');
    text.push_str(&"  ".repeat(level));
}

/// Writes a string as RFC 8785 section 3.2.2.2 states: only `"`, `\` and control characters are
/// escaped, with the short forms where JSON has them and `\u00xx` in lowercase otherwise.
fn write_string(text: &mut String, value: &str) {
    text.push('"');
    for character in value.chars() {
        match character {
            '"' => text.push_str("\\\""),
            '\\' => text.push_str("\\\\"),
            '\u{8}' => text.push_str("\\b"),
            '\t' => text.push_str("\\t"),
            '\n' => text.push_str("\\n"),
            '\u{c}' => text.push_str("\\f"),
            '\r' => text.push_str("\\r"),
            control if u32::from(control) < 0x20 => {
                let _ = write!(text, "\\u{:04x}", u32::from(control));
            }
            other => text.push(other),
        }
    }
    text.push('"');
}

/// The text of a number as RFC 8785 section 3.2.2.3 states: the shortest digits that read back
/// as the same double, laid out as ECMAScript `Number.prototype.toString` lays them out.
fn number_text(value: f64) -> String {
    if value == 0.0 {
        return "0".to_owned();
    }
    if value < 0.0 {
        return format!("-{}", number_text(-value));
    }
    let scientific = format!("{value:e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent: i64 = exponent.parse().unwrap_or(0);
    layout(&digits, exponent + 1)
}

/// Lays out the significant `digits` of a positive number whose decimal point follows the
/// first `point` digits (ECMAScript `Number::toString`, steps 6 to 10).
fn layout(digits: &str, point: i64) -> String {
    let count = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    let zeros = |amount: i64| "0".repeat(usize::try_from(amount).unwrap_or(0));
    if count <= point && point <= 21 {
        return format!("{digits}{}", zeros(point - count));
    }
    if 0 < point && point <= 21 {
        let (whole, fraction) = digits.split_at(usize::try_from(point).unwrap_or(0));
        return format!("{whole}.{fraction}");
    }
    if -6 < point && point <= 0 {
        return format!("0.{}{digits}", zeros(-point));
    }
    let exponent = point - 1;
    let sign = if exponent < 0 { '-' } else { '+' };
    let (first, rest) = digits.split_at(1);
    if rest.is_empty() {
        format!("{first}e{sign}{}", exponent.abs())
    } else {
        format!("{first}.{rest}e{sign}{}", exponent.abs())
    }
}

/// Orders member names by their UTF-16 code units (RFC 8785 section 3.2.3).
fn utf16_order(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

impl<'de> Deserialize<'de> for JsonValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(JsonVisitor)
    }
}

struct JsonVisitor;

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = JsonValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<JsonValue, E> {
        Ok(JsonValue::Null)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<JsonValue, E> {
        Ok(JsonValue::Bool(value))
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "RFC 8785 section 3.2.2.3 reads every number as a double"
    )]
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<JsonValue, E> {
        Ok(JsonValue::Number(value as f64))
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "RFC 8785 section 3.2.2.3 reads every number as a double"
    )]
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<JsonValue, E> {
        Ok(JsonValue::Number(value as f64))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<JsonValue, E> {
        Ok(JsonValue::Number(value))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<JsonValue, E> {
        Ok(JsonValue::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<JsonValue, E> {
        Ok(JsonValue::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<JsonValue, A::Error> {
        let mut elements = Vec::new();
        while let Some(element) = sequence.next_element()? {
            elements.push(element);
        }
        Ok(JsonValue::Array(elements))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<JsonValue, A::Error> {
        let mut members: Vec<(String, JsonValue)> = Vec::new();
        while let Some((name, value)) = map.next_entry::<String, JsonValue>()? {
            match members.binary_search_by(|(member, _)| utf16_order(member, &name)) {
                Ok(_) => {
                    return Err(de::Error::custom(format!(
                        "the member name `{name}` is repeated"
                    )));
                }
                Err(position) => members.insert(position, (name, value)),
            }
        }
        Ok(JsonValue::Object(members))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canonical(text: &str) -> String {
        JsonValue::parse(text).unwrap().canonical()
    }

    #[test]
    fn cs_2_10_sorts_members_by_utf16_code_units() {
        let text = r#"{"\u20ac": 1, "\r": 2, "\ud83d\ude00": 3, "1": 4, "\u0080": 5, "a": {"b": 1, "a": 2}}"#;
        assert_eq!(
            canonical(text),
            "{\"\\r\":2,\"1\":4,\"a\":{\"a\":2,\"b\":1},\"\u{80}\":5,\"\u{1f600}\":3,\"\u{20ac}\":1}"
        );
    }

    #[test]
    fn cs_2_10_writes_numbers_as_rfc_8785() {
        let cases = [
            ("0", "0"),
            ("-0", "0"),
            ("1.0", "1"),
            ("45.00", "45"),
            ("-1.5", "-1.5"),
            ("1e21", "1e+21"),
            ("1e20", "100000000000000000000"),
            ("123456789012345678901234", "1.2345678901234568e+23"),
            ("0.000001", "0.000001"),
            ("1e-7", "1e-7"),
            ("4.5e-7", "4.5e-7"),
            ("333333333.33333329", "333333333.3333333"),
            ("9007199254740993", "9007199254740992"),
            ("5e-324", "5e-324"),
        ];
        for (number, expected) in cases {
            assert_eq!(canonical(number), expected, "{number}");
        }
    }

    #[test]
    fn cs_2_10_escapes_only_quotes_backslashes_and_control_characters() {
        assert_eq!(
            canonical(r#""a\"b\\c\/d\u0001\u001f\n\u007f\u00e9""#),
            "\"a\\\"b\\\\c/d\\u0001\\u001f\\n\u{7f}\u{e9}\""
        );
    }

    #[test]
    fn cs_2_10_rejects_a_duplicate_member_name() {
        let error = JsonValue::parse(r#"{"a": {"b": 1, "b": 2}}"#).unwrap_err();
        assert!(error.contains("`b` is repeated"), "{error}");
    }

    #[test]
    fn cs_2_10_rejects_invalid_json() {
        assert!(JsonValue::parse("{\"a\": 1,}").is_err());
        assert!(JsonValue::parse("[1] [2]").is_err());
    }

    #[test]
    fn cs_2_10_writes_the_stored_form() {
        let value = JsonValue::parse(r#"{"b": [1, {"d": [], "c": {}}], "a": "x"}"#).unwrap();
        assert_eq!(
            value.stored_form(),
            "{\n  \"a\": \"x\",\n  \"b\": [\n    1,\n    {\n      \"c\": {},\n      \"d\": []\n    }\n  ]\n}\n"
        );
        assert_eq!(JsonValue::Array(Vec::new()).stored_form(), "[]\n");
    }

    #[test]
    fn cs_2_10_the_stored_form_and_canonical_json_read_back_equal() {
        let value = JsonValue::parse(r#"{"b": [1.5, null, true], "a": "\u0000"}"#).unwrap();
        let stored = JsonValue::parse(&value.stored_form()).unwrap();
        assert_eq!(stored.canonical(), value.canonical());
    }

    #[test]
    fn cs_2_10_reads_a_serializable_output() {
        let value = JsonValue::of(&serde_json::json!({"z": 1, "a": [2]})).unwrap();
        assert_eq!(value.canonical(), "{\"a\":[2],\"z\":1}");
        assert_eq!(value.member("z"), Some(&JsonValue::Number(1.0)));
        assert_eq!(value.without(&["z"]).canonical(), "{\"a\":[2]}");
    }
}
