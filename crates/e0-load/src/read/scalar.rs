//! Construction of one scalar from its text, its style and its tag (E0A-2.2, E0A-2.5, E0A-2.8).
//!
//! Only the standard tags `!!str`, `!!null`, `!!bool`, `!!int`, `!!float` and `!!timestamp`
//! construct a scalar; every other tag is an error, so no arbitrary object is ever built.
//! Rejecting `!!binary`, `!!set`, `!!omap` and `!!pairs`, whose values are not JSON values, is
//! provisional until the specification decides (#61).

use super::compose::RawTag;
use super::numbers::{construct_boolean, construct_float, construct_integer};
use super::resolve::{ImplicitKind, resolve};
use super::timestamp::timestamp_text;
use super::tree::Value;

/// Where a scalar appears: the value `=` is a string as a key and an error as a value, as in the
/// reference loader.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScalarPosition {
    Key,
    Value,
}

/// Constructs a scalar. `plain` is false for quoted and block scalars, which are never resolved
/// implicitly.
///
/// # Errors
///
/// A description when the tag has no scalar constructor or the text is not a value of its tag.
pub(crate) fn construct_scalar(
    text: &str,
    plain: bool,
    tag: &RawTag,
    position: ScalarPosition,
) -> Result<Value, String> {
    match tag {
        RawTag::Absent if plain => construct_implicit(text, position),
        RawTag::Absent | RawTag::NonSpecific => Ok(Value::String(text.to_owned())),
        RawTag::Standard(suffix) => construct_tagged(text, suffix),
        RawTag::Other(name) => Err(format!("no constructor for the tag '{name}'")),
    }
}

fn construct_implicit(text: &str, position: ScalarPosition) -> Result<Value, String> {
    match resolve(text) {
        ImplicitKind::Boolean(value) => Ok(Value::Boolean(value)),
        ImplicitKind::Null => Ok(Value::Null),
        ImplicitKind::Integer => construct_tagged(text, "int"),
        ImplicitKind::Float => construct_tagged(text, "float"),
        ImplicitKind::Timestamp => construct_tagged(text, "timestamp"),
        ImplicitKind::Value if position == ScalarPosition::Key => {
            Ok(Value::String(text.to_owned()))
        }
        ImplicitKind::Value => Err("no constructor for the value '='".to_owned()),
        ImplicitKind::Merge => Err("the merge key '<<' is not a value".to_owned()),
        ImplicitKind::String => Ok(Value::String(text.to_owned())),
    }
}

fn construct_tagged(text: &str, suffix: &str) -> Result<Value, String> {
    let invalid = || format!("'{text}' is not a valid !!{suffix}");
    match suffix {
        "str" => Ok(Value::String(text.to_owned())),
        "null" => Ok(Value::Null),
        "bool" => construct_boolean(text)
            .map(Value::Boolean)
            .ok_or_else(invalid),
        "int" => construct_integer(text)
            .map(Value::Integer)
            .ok_or_else(invalid),
        "float" => construct_float(text).map(Value::Float).ok_or_else(invalid),
        "timestamp" => timestamp_text(text).map(Value::String),
        _ => Err(format!("no scalar constructor for the tag !!{suffix}")),
    }
}
