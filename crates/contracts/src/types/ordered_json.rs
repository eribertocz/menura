//! A JSON value whose objects keep their members in the order they were read (MIG-3.3).
//!
//! A [`serde_json::Value`] object orders its members by name, so it cannot carry a model
//! document as it was written. [`OrderedJson`] keeps the order of every object, and serializes
//! and deserializes in that order.

use std::fmt;

use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};

/// A JSON value whose objects keep the order of their members.
///
/// # Example
///
/// ```
/// use menura_contracts::types::ordered_json::{OrderedJson, OrderedObject};
///
/// let object: OrderedObject = serde_json::from_str(r#"{"b": 1, "a": [true, null]}"#)?;
/// assert_eq!(object.members()[0].0, "b");
/// assert_eq!(serde_json::to_string(&object)?, r#"{"b":1,"a":[true,null]}"#);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum OrderedJson {
    /// `null`.
    Null,
    /// `true` or `false`.
    Boolean(bool),
    /// An integer of any size, kept as its exact digits.
    Integer(JsonInteger),
    /// A finite number that is not an integer.
    Float(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<OrderedJson>),
    /// An object.
    Object(OrderedObject),
}

/// An integer of any size, kept as its canonical decimal digits: an optional `-`, then digits
/// without leading zeros; zero is `0`, never negative.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonInteger(String);

impl JsonInteger {
    /// The integer written `text`, when `text` is canonical decimal digits.
    ///
    /// ```
    /// use menura_contracts::types::ordered_json::JsonInteger;
    ///
    /// let big = JsonInteger::parse("-123456789012345678901234567890123456789012").unwrap();
    /// assert_eq!(big.as_str(), "-123456789012345678901234567890123456789012");
    /// assert!(JsonInteger::parse("012").is_none());
    /// assert!(JsonInteger::parse("-0").is_none());
    /// ```
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let digits = text.strip_prefix('-').unwrap_or(text);
        let canonical = !digits.is_empty()
            && digits.bytes().all(|byte| byte.is_ascii_digit())
            && (digits == "0" || !digits.starts_with('0'))
            && !(digits == "0" && text.starts_with('-'));
        canonical.then(|| Self(text.to_owned()))
    }

    /// The digits, with a leading `-` when the integer is negative.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<i128> for JsonInteger {
    fn from(value: i128) -> Self {
        Self(value.to_string())
    }
}

/// A JSON object whose members keep their order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderedObject {
    members: Vec<(String, OrderedJson)>,
}

impl OrderedObject {
    /// The object with `members`, in that order.
    #[must_use]
    pub fn new(members: Vec<(String, OrderedJson)>) -> Self {
        Self { members }
    }

    /// The members, in order.
    #[must_use]
    pub fn members(&self) -> &[(String, OrderedJson)] {
        &self.members
    }

    /// The members, in order, to change their values in place.
    pub fn members_mut(&mut self) -> &mut [(String, OrderedJson)] {
        &mut self.members
    }

    /// The value of the first member named `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&OrderedJson> {
        self.members
            .iter()
            .find(|(member, _)| member == name)
            .map(|(_, value)| value)
    }
}

impl Serialize for OrderedJson {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Boolean(value) => serializer.serialize_bool(*value),
            // A serializer writes numbers of at most 128 bits; the exact text of a larger one
            // has its own writer in `menura-core`.
            Self::Integer(value) => match value.as_str().parse::<i128>() {
                Ok(number) => serializer.serialize_i128(number),
                Err(_) => Err(serde::ser::Error::custom(format!(
                    "the integer {} has more than 128 bits",
                    value.as_str()
                ))),
            },
            Self::Float(value) => serializer.serialize_f64(*value),
            Self::String(text) => serializer.serialize_str(text),
            Self::Array(items) => {
                let mut sequence = serializer.serialize_seq(Some(items.len()))?;
                for item in items {
                    sequence.serialize_element(item)?;
                }
                sequence.end()
            }
            Self::Object(object) => object.serialize(serializer),
        }
    }
}

impl Serialize for OrderedObject {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.members.len()))?;
        for (name, value) in &self.members {
            map.serialize_entry(name, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for OrderedJson {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(OrderedJsonVisitor)
    }
}

impl<'de> Deserialize<'de> for OrderedObject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match OrderedJson::deserialize(deserializer)? {
            OrderedJson::Object(object) => Ok(object),
            _ => Err(de::Error::custom("expected a JSON object")),
        }
    }
}

/// Builds an [`OrderedJson`] from the events of a deserializer.
struct OrderedJsonVisitor;

impl<'de> Visitor<'de> for OrderedJsonVisitor {
    type Value = OrderedJson;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value")
    }

    fn visit_unit<E: de::Error>(self) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Null)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<OrderedJson, D::Error> {
        OrderedJson::deserialize(deserializer)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Boolean(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Integer(JsonInteger::from(i128::from(value))))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Integer(JsonInteger::from(i128::from(value))))
    }

    fn visit_i128<E: de::Error>(self, value: i128) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Integer(JsonInteger::from(value)))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<OrderedJson, E> {
        Ok(OrderedJson::Float(value))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<OrderedJson, E> {
        Ok(OrderedJson::String(value.to_owned()))
    }

    fn visit_string<E: de::Error>(self, value: String) -> Result<OrderedJson, E> {
        Ok(OrderedJson::String(value))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<OrderedJson, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = access.next_element()? {
            items.push(item);
        }
        Ok(OrderedJson::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<OrderedJson, A::Error> {
        let mut members = Vec::new();
        while let Some(member) = access.next_entry::<String, OrderedJson>()? {
            members.push(member);
        }
        Ok(OrderedJson::Object(OrderedObject::new(members)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn objects_keep_their_member_order_both_ways() {
        let text = r#"{"z":{"y":1,"x":-2},"a":[1.5,"s",false,null]}"#;
        let value: OrderedJson = serde_json::from_str(text).unwrap();
        assert_eq!(serde_json::to_string(&value).unwrap(), text);
    }

    #[test]
    fn integers_beyond_64_bits_are_written_with_their_digits() {
        let value = OrderedJson::Integer(JsonInteger::from(i128::MIN));
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            "-170141183460469231731687303715884105728"
        );
        let larger = OrderedJson::Integer(JsonInteger::parse("1".repeat(40).as_str()).unwrap());
        assert!(serde_json::to_string(&larger).is_err());
    }

    #[test]
    fn a_value_that_is_not_an_object_is_no_ordered_object() {
        assert!(serde_json::from_str::<OrderedObject>("[]").is_err());
    }
}
