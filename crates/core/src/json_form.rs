//! The JSON value of a [`JsonForm`], the form in which the `model` crate defines normalized
//! trees and `ai_context` values.

use menura_model::json::JsonForm;
use serde_json::{Map, Number, Value};

/// `form` as a JSON value. A float whose text is not a finite number becomes its text.
pub(crate) fn json_value(form: &JsonForm) -> Value {
    match form {
        JsonForm::Null => Value::Null,
        JsonForm::Boolean(value) => Value::Bool(*value),
        JsonForm::Integer(value) => Value::from(*value),
        JsonForm::Number(text) => text
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map_or_else(|| Value::String(text.clone()), Value::Number),
        JsonForm::Text(text) => Value::String(text.clone()),
        JsonForm::List(items) => Value::Array(items.iter().map(json_value).collect()),
        JsonForm::Object(members) => Value::Object(
            members
                .iter()
                .map(|(name, value)| (name.clone(), json_value(value)))
                .collect::<Map<String, Value>>(),
        ),
    }
}
