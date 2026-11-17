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
/// A repeated member name is checked first (`E0C-06`, E0C-2.7), then the version (`E0C-01`,
/// E0C-2.1). Either one stops the checks of the entry: with a repeated member `data` has more
/// than one reading, and the version decides which keys exist. Otherwise every key
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
    if let Some(name) = &data.repeated_member {
        let message = format!("the member name `{name}` is repeated inside data");
        return Err(vec![diagnostic(ExtensionCode::RepeatedMember, message)]);
    }
    if let Some(message) = version_problem(data.members.get(VERSION)) {
        return Err(vec![diagnostic(ExtensionCode::Version, message)]);
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

/// Why `version` is not acceptable (E0C-2.1): it is missing, or it is not the JSON integer 1,
/// whatever its JSON type.
fn version_problem(version: Option<&Json>) -> Option<String> {
    match version {
        None => Some("data.version is missing".to_owned()),
        Some(Json::Number(number))
            if number.is_u64() && number.as_u64() == Some(SUPPORTED_VERSION) =>
        {
            None
        }
        Some(other) => Some(format!(
            "data.version {other} is not supported; the supported version is the integer {SUPPORTED_VERSION}"
        )),
    }
}
