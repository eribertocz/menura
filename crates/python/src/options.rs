//! The options arguments: a `dict` converted as JSON and read by `core` as the CLI reads its
//! option files (ARC-6.4).

use menura_core::{
    CompileOptions, LoadOptions, UsageError, parse_compile_options, parse_load_options,
};
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyFloat, PyInt, PyString};
use serde_json::{Map, Number, Value};

/// The load options of `options`; `None` gives the defaults.
///
/// # Errors
///
/// The core's usage error about `load_options` when `options` is not an object of load options.
pub(crate) fn load_options(options: Option<&Bound<'_, PyAny>>) -> Result<LoadOptions, UsageError> {
    match options {
        Some(options) => parse_load_options(&json_bytes(options)),
        None => Ok(LoadOptions::default()),
    }
}

/// The compile options of `options`; `None` gives the defaults.
///
/// # Errors
///
/// The core's usage error about `compile_options` when `options` is not an object of compile
/// options.
pub(crate) fn compile_options(
    options: Option<&Bound<'_, PyAny>>,
) -> Result<CompileOptions, UsageError> {
    match options {
        Some(options) => parse_compile_options(&json_bytes(options)),
        None => Ok(CompileOptions::default()),
    }
}

/// The JSON text of `options`. A `dict` whose keys are all `str` becomes an object; anything
/// else has no object form and becomes an array, which `core` refuses as not an object.
fn json_bytes(options: &Bound<'_, PyAny>) -> Vec<u8> {
    let value = options
        .cast::<PyDict>()
        .ok()
        .and_then(|dictionary| object(dictionary))
        .map_or_else(|| Value::Array(Vec::new()), Value::Object);
    value.to_string().into_bytes()
}

/// The members of `dictionary`, or `None` when a key is not a `str`.
fn object(dictionary: &Bound<'_, PyDict>) -> Option<Map<String, Value>> {
    let mut members = Map::new();
    for (key, value) in dictionary.iter() {
        let key = key.cast::<PyString>().ok()?.to_str().ok()?.to_owned();
        members.insert(key, scalar(&value));
    }
    Some(members)
}

/// The JSON form of an option value. A `bool` is only a boolean, an `int` that is not a `bool`
/// is an integer, a `float` is a number with a fraction and `None` is `null`; a value without
/// such a form becomes an array, which no option accepts.
fn scalar(value: &Bound<'_, PyAny>) -> Value {
    if let Ok(boolean) = value.cast::<PyBool>() {
        return Value::Bool(boolean.is_true());
    }
    if value.is_none() {
        return Value::Null;
    }
    if let Ok(text) = value.cast::<PyString>() {
        return text.to_str().map_or_else(
            |_| Value::Array(Vec::new()),
            |text| Value::String(text.to_owned()),
        );
    }
    if value.is_instance_of::<PyInt>() {
        return integer(value);
    }
    if let Ok(float) = value.cast::<PyFloat>() {
        return Number::from_f64(float.value()).map_or(Value::Null, Value::Number);
    }
    Value::Array(Vec::new())
}

/// An `int` as a JSON integer; one outside the 64-bit range is a number with a fraction, as
/// `core` reads such an integer in an option file.
fn integer(value: &Bound<'_, PyAny>) -> Value {
    if let Ok(signed) = value.extract::<i64>() {
        return Value::from(signed);
    }
    if let Ok(unsigned) = value.extract::<u64>() {
        return Value::from(unsigned);
    }
    value
        .extract::<f64>()
        .ok()
        .and_then(Number::from_f64)
        .map_or(Value::Null, Value::Number)
}
