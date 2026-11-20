//! SEC-3.6: the context fingerprint, the CON-2 hash of `{roles, attributes}` after CON-4
//! encoding.

use std::collections::BTreeMap;

use menura_contracts::{Sha256Hash, encode_integer, hash_value};
use serde_json::{Map, Value};

use super::value::{AdmittedScalar, AdmittedValue};
use crate::role::Role;

/// The fingerprint of the declared `roles` (already in CON-5 order, without duplicates) and
/// the declared `attributes`, each list sorted by value in its type without duplicates.
pub(crate) fn fingerprint(
    roles: &[Role],
    attributes: &BTreeMap<String, AdmittedValue>,
) -> Sha256Hash {
    let roles = roles
        .iter()
        .map(|role| Value::String(role.as_str().to_owned()))
        .collect();
    let attributes: Map<String, Value> = attributes
        .iter()
        .map(|(name, value)| (name.clone(), encode_value(value)))
        .collect();
    let mut document = Map::new();
    document.insert("roles".to_owned(), Value::Array(roles));
    document.insert("attributes".to_owned(), Value::Object(attributes));
    hash_value(&Value::Object(document))
}

fn encode_value(value: &AdmittedValue) -> Value {
    match value {
        AdmittedValue::Scalar(scalar) => encode_scalar(scalar),
        AdmittedValue::List(items) => {
            let mut sorted: Vec<&AdmittedScalar> = items.iter().collect();
            sorted.sort();
            sorted.dedup();
            Value::Array(sorted.into_iter().map(encode_scalar).collect())
        }
    }
}

/// The CON-4 encoding of one value.
fn encode_scalar(scalar: &AdmittedScalar) -> Value {
    match scalar {
        AdmittedScalar::Text(text) => Value::String(text.clone()),
        AdmittedScalar::Integer(integer) => encode_integer(i128::from(*integer)),
        AdmittedScalar::Decimal(decimal) => Value::String(decimal.to_string()),
        AdmittedScalar::Date(date) => Value::String(date.to_string()),
        AdmittedScalar::Boolean(boolean) => Value::Bool(*boolean),
    }
}
