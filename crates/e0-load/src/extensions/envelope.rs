//! The common rules for an entry of ours: its version and the placement of its keys (E0C-2.1,
//! E0C-2.3, E0C-2.4, MDL-2.2).

use serde_json::{Map, Value as Json};

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::entry::OwnData;
use super::keys::{KeyKind, SUPPORTED_VERSION, VERSION, key_kind};
use super::output::ObjectKind;

/// The extension keys of `data`, without `namespace` and `version`, or the diagnostics that
/// reject the entry.
///
/// The version is checked first: when it is missing, of the wrong type or not supported, the
/// keys are not checked, because the version decides which keys exist. Otherwise every key
/// that no extension defines is reported once as `E0C-02`, and every key on the wrong object
/// type once as `E0C-05`, each diagnostic naming its keys in order.
pub(crate) fn extension_keys(
    data: OwnData<'_>,
    object: ObjectKind,
) -> Result<Map<String, Json>, Vec<ExtensionDiagnostic>> {
    let diagnostic = |code: ExtensionCode, message: String| ExtensionDiagnostic {
        code,
        pointer: data.pointer.clone(),
        location: data.node.location,
        message,
    };
    if let Some((code, message)) = version_problem(data.members.get(VERSION)) {
        return Err(vec![diagnostic(code, message)]);
    }
    let mut unknown = Vec::new();
    let mut misplaced = Vec::new();
    let mut extensions = Map::new();
    for (key, value) in data.members {
        match key_kind(&key) {
            KeyKind::Envelope => {}
            KeyKind::Extension(valid_on) if valid_on == object => {
                extensions.insert(key, value);
            }
            KeyKind::Extension(valid_on) => {
                misplaced.push(format!("`{key}` is valid only on a {}", valid_on.as_str()));
            }
            KeyKind::Unknown => unknown.push(format!("`{key}`")),
        }
    }
    let mut errors = Vec::new();
    if !unknown.is_empty() {
        let message = format!("no extension defines the key {}", unknown.join(", "));
        errors.push(diagnostic(ExtensionCode::UnknownKey, message));
    }
    if !misplaced.is_empty() {
        let message = format!(
            "extension key on a {}: {}",
            object.as_str(),
            misplaced.join("; ")
        );
        errors.push(diagnostic(ExtensionCode::WrongObject, message));
    }
    if errors.is_empty() {
        Ok(extensions)
    } else {
        Err(errors)
    }
}

/// Why `version` is not acceptable (E0C-2.1): missing or not supported is `E0C-01`, a value
/// that is not an integer is `E0C-03`.
fn version_problem(version: Option<&Json>) -> Option<(ExtensionCode, String)> {
    let Some(version) = version else {
        return Some((ExtensionCode::Version, "data.version is missing".to_owned()));
    };
    match version {
        Json::Number(number) if number.as_u64() == Some(SUPPORTED_VERSION) => None,
        Json::Number(number) if number.is_u64() || number.is_i64() => Some((
            ExtensionCode::Version,
            format!(
                "data.version {number} is not supported; the supported version is {SUPPORTED_VERSION}"
            ),
        )),
        other => Some((
            ExtensionCode::WrongType,
            format!("data.version must be an integer, not {}", json_type(other)),
        )),
    }
}

fn json_type(value: &Json) -> &'static str {
    match value {
        Json::Null => "null",
        Json::Bool(_) => "a boolean",
        Json::Number(_) => "a number with a fraction or exponent",
        Json::String(_) => "a string",
        Json::Array(_) => "an array",
        Json::Object(_) => "an object",
    }
}
