//! Reading the JSON text of `data` without losing repeated member names (E0C-2.7).
//!
//! A JSON reader keeps one value of a repeated member name and drops the others silently. The
//! text is read here with a `serde` visitor over the `serde_json` reader instead, which builds
//! the same value and also reports what a plain value cannot keep: the first repeated member
//! name, and every value of the top-level `namespace` member (E0C-1.3).

use std::collections::BTreeSet;
use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value as Json};

use super::keys::NAMESPACE;

/// A JSON value read from the text of `data`, with what a plain JSON value cannot keep.
#[derive(Debug, PartialEq)]
pub(crate) struct Members {
    /// The value. When a member name is repeated in an object, the last value is kept, as the
    /// JSON reader keeps it.
    pub(crate) value: Json,
    /// The first member name, in text order, repeated in one object anywhere in the value.
    pub(crate) repeated: Option<String>,
    /// When the value is an object, every value of its own `namespace` member, in text order.
    pub(crate) namespaces: Vec<Json>,
}

impl Members {
    fn plain(value: Json) -> Self {
        Self {
            value,
            repeated: None,
            namespaces: Vec::new(),
        }
    }
}

/// Reads `text` as one JSON value.
///
/// # Errors
///
/// Returns the JSON reader's error when `text` is not valid JSON.
pub(crate) fn read_members(text: &str) -> Result<Members, serde_json::Error> {
    serde_json::from_str(text)
}

impl<'de> Deserialize<'de> for Members {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(MembersVisitor)
    }
}

/// Builds [`Members`] from the events of the JSON reader.
struct MembersVisitor;

impl<'de> Visitor<'de> for MembersVisitor {
    type Value = Members;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Members, E> {
        Ok(Members::plain(Json::Bool(value)))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Members, E> {
        Ok(Members::plain(Json::Number(value.into())))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Members, E> {
        Ok(Members::plain(Json::Number(value.into())))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Members, E> {
        Number::from_f64(value)
            .map(|number| Members::plain(Json::Number(number)))
            .ok_or_else(|| E::custom("a number that is not finite"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Members, E> {
        Ok(Members::plain(Json::String(value.to_owned())))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<Members, E> {
        Ok(Members::plain(Json::String(value)))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Members, E> {
        Ok(Members::plain(Json::Null))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Members, A::Error> {
        let mut items = Vec::new();
        let mut repeated = None;
        while let Some(item) = sequence.next_element::<Members>()? {
            repeated = repeated.or(item.repeated);
            items.push(item.value);
        }
        Ok(Members {
            repeated,
            ..Members::plain(Json::Array(items))
        })
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<Members, A::Error> {
        let mut members = Map::new();
        let mut names = BTreeSet::new();
        let mut repeated = None;
        let mut namespaces = Vec::new();
        while let Some(name) = object.next_key::<String>()? {
            if !names.insert(name.clone()) {
                repeated = repeated.or_else(|| Some(name.clone()));
            }
            let member: Members = object.next_value()?;
            repeated = repeated.or(member.repeated);
            if name == NAMESPACE {
                namespaces.push(member.value.clone());
            }
            members.insert(name, member.value);
        }
        Ok(Members {
            value: Json::Object(members),
            repeated,
            namespaces,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::read_members;

    fn repeated(text: &str) -> Option<String> {
        read_members(text).expect("valid JSON").repeated
    }

    #[test]
    fn e0c_2_7_repeated_names_are_found_at_any_depth() {
        assert_eq!(repeated(r#"{"a": 1, "b": 2}"#), None);
        assert_eq!(repeated(r#"{"a": 1, "a": 2}"#).as_deref(), Some("a"));
        assert_eq!(
            repeated(r#"[1, "x", {"o": {"p": [true, null], "p": -1.5e3}}]"#).as_deref(),
            Some("p")
        );
        assert_eq!(
            repeated(r#"{"a": {"a": 1}, "b": [{"a": 1}, {"a": 2}]}"#),
            None
        );
        assert_eq!(
            repeated(r#"{"z": 0, "z": {"y": 1, "y": 2}}"#).as_deref(),
            Some("z")
        );
        assert_eq!(
            repeated(r#"{"z": {"y": 1, "y": 2}, "z": 0}"#).as_deref(),
            Some("y")
        );
    }

    #[test]
    fn e0c_2_7_names_are_compared_after_decoding_escapes() {
        let escape = |hex: &str| format!("{}u{hex}", '\\');
        let letter = format!(r#"{{"a": 1, "{}": 2}}"#, escape("0061"));
        assert_eq!(repeated(&letter).as_deref(), Some("a"));
        let pair = format!(
            r#"{{"{}": 1, "{}{}": 2}}"#,
            '\u{1F600}',
            escape("d83d"),
            escape("de00")
        );
        assert_eq!(repeated(&pair).as_deref(), Some("\u{1F600}"));
        assert_eq!(
            repeated(r#"{"q\"\\\/\b\f\n\r\t": 1, "q\"\\/\b\f\n\r\t": 2}"#).as_deref(),
            Some("q\"\\/\u{8}\u{c}\n\r\t")
        );
        assert_eq!(repeated(r#"{"a]}": "}{,", "b": "a"}"#), None);
    }

    #[test]
    fn the_value_is_the_json_reader_value() {
        let text = r#"{"n": [1, -2, 1.5, 18446744073709551615, true, null, "t"], "k": 1, "k": 2}"#;
        let expected: serde_json::Value = serde_json::from_str(text).expect("valid JSON");
        assert_eq!(read_members(text).expect("valid JSON").value, expected);
        assert!(read_members("{\"a\": 1,}").is_err());
        assert!(read_members("not json").is_err());
    }

    #[test]
    fn e0c_1_3_every_top_level_namespace_value_is_kept() {
        let text = r#"{"namespace": "acme", "x": {"namespace": "inner"}, "namespace" : "menura" }"#;
        let members = read_members(text).expect("valid JSON");
        assert_eq!(members.repeated.as_deref(), Some("namespace"));
        assert_eq!(members.namespaces, [json!("acme"), json!("menura")]);
    }
}
