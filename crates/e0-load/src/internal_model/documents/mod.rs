//! The JSON documents the hashes cover (E0H-3): meaning only, never source locations, warnings
//! or messages (E0H-3.4). Trees use the JSON form the `model` crate defines.

mod elements;
mod metrics;

pub(super) use elements::{
    dataset, descriptions, field_definition, population, relationship, relationship_definition,
    security_context,
};
pub(super) use metrics::{metric, metric_definition};

use menura_model::expression::Expression;
use menura_model::internal_model::{DiagnosticCode, NormalizedName, TypedValue, ValueType};
use menura_model::json::JsonForm;
use serde::Serialize;
use serde_json::{Map, Number, Value as Json};

/// The JSON value of `form`. A float whose text is not a finite JSON number is kept as its text.
pub(super) fn json(form: &JsonForm) -> Json {
    match form {
        JsonForm::Null => Json::Null,
        JsonForm::Boolean(value) => Json::Bool(*value),
        JsonForm::Integer(value) => Json::Number(Number::from(*value)),
        JsonForm::Number(text) => text
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map_or_else(|| Json::String(text.clone()), Json::Number),
        JsonForm::Text(text) => Json::String(text.clone()),
        JsonForm::List(items) => Json::Array(items.iter().map(json).collect()),
        JsonForm::Object(members) => Json::Object(
            members
                .iter()
                .map(|(name, value)| (name.clone(), json(value)))
                .collect::<Map<String, Json>>(),
        ),
    }
}

/// The JSON form of an optional tree.
pub(super) fn tree(expression: Option<&Expression>) -> Json {
    expression.map_or(Json::Null, |expression| json(&expression.json_form()))
}

/// A normalized name.
pub(super) fn name(name: &NormalizedName) -> Json {
    Json::String(name.as_str().to_owned())
}

/// A list of normalized names, in the given order.
pub(super) fn names<'name>(names: impl IntoIterator<Item = &'name NormalizedName>) -> Json {
    Json::Array(names.into_iter().map(name).collect())
}

/// Query error codes.
pub(super) fn codes(codes: &[DiagnosticCode]) -> Json {
    Json::Array(
        codes
            .iter()
            .map(|code| Json::String(code.as_str().to_owned()))
            .collect(),
    )
}

/// A type and nullability: `{"type": <logical type or "null">, "nullable": …}`.
pub(super) fn typed(typed: Option<&TypedValue>) -> Json {
    typed.map_or(Json::Null, |typed| {
        let value_type = match &typed.value_type {
            ValueType::Null => Json::String("null".to_owned()),
            ValueType::Logical(logical) => serialized(logical),
        };
        object([
            ("type", value_type),
            ("nullable", Json::Bool(typed.nullable)),
        ])
    })
}

/// The serialized form of a contract value, such as a logical type or a grain.
pub(super) fn serialized(value: &impl Serialize) -> Json {
    serde_json::to_value(value).unwrap_or(Json::Null)
}

/// An object with `members`.
pub(super) fn object<const COUNT: usize>(members: [(&str, Json); COUNT]) -> Json {
    Json::Object(
        members
            .into_iter()
            .map(|(member, value)| (member.to_owned(), value))
            .collect(),
    )
}

/// A string.
pub(super) fn text(text: &str) -> Json {
    Json::String(text.to_owned())
}
