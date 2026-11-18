//! The common rules for an entry of ours: its reading, its version and the placement of its
//! keys (E0C-2.1, E0C-2.3, E0C-2.4, E0C-2.7, E0C-2.8, MDL-2.2).

use serde_json::{Map, Value as Json};

use super::diagnostic::{ExtensionCode, ExtensionDiagnostic};
use super::entry::OwnData;
use super::keys::{KeyKind, SUPPORTED_VERSION, VERSION, key_kind};
use super::output::ObjectKind;

/// The extension keys of `data`, without `namespace` and `version`, or the diagnostics that
/// reject the entry.
///
/// A repeated member name is checked first (`E0C-06`, E0C-2.7), then a development-line `data`
/// without a JSON form (`E0C-03`, E0C-2.8), then the version (`E0C-01`, E0C-2.1). Each one
/// stops the checks of the entry: with a repeated member `data` has more than one reading,
/// without a JSON form it has no JSON reading for the other rules to apply to, and the version
/// decides which keys exist. Otherwise every key
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
    if let Some((code, message)) = reading_problem(&data) {
        return Err(vec![diagnostic(code, message)]);
    }
    let mut unknown = Vec::new();
    let mut misplaced = Vec::new();
    let mut extensions = Map::new();
    for (key, value) in data.members {
        match key_kind(&key) {
            KeyKind::Envelope => {}
            KeyKind::Extension(valid_on) if valid_on.contains(&object) => {
                extensions.insert(key, value);
            }
            KeyKind::Extension(valid_on) => {
                let names: Vec<&str> = valid_on.iter().map(|kind| kind.as_str()).collect();
                misplaced.push(format!(
                    "`{key}` is valid only on a {}",
                    names.join(" or a ")
                ));
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

/// The problem that stops the checks of `data`, in the order they are checked: a repeated
/// member name (`E0C-06`), no JSON form (`E0C-03`), then the version (`E0C-01`).
fn reading_problem(data: &OwnData<'_>) -> Option<(ExtensionCode, String)> {
    if let Some(name) = &data.repeated_member {
        let message = format!("the member name `{name}` is repeated inside data");
        return Some((ExtensionCode::RepeatedMember, message));
    }
    if let Some(reason) = &data.without_json_form {
        return Some((
            ExtensionCode::WrongValue,
            format!("data has no JSON form: {reason}"),
        ));
    }
    version_problem(data.members.get(VERSION)).map(|message| (ExtensionCode::Version, message))
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
