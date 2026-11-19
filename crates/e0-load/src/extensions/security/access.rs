//! Reading one `access` value (SCY-3, SCY-5.1.3, `E0C-44`).

use semantic_compiler_security::{AccessMode, DatasetAccess, FieldRestriction, Role};
use serde_json::{Map, Value as Json};

use super::values::{
    DeclaredRoles, ListRule, has_repeats, object, required, roles, strings, wrong_value,
};
use crate::extensions::diagnostic::ExtensionCode;
use crate::extensions::problems::Problems;

const MODE: &str = "mode";
const ROLES: &str = "roles";
const FIELDS: &str = "fields";
const NAMES: &str = "names";

/// The values of `mode` (SCY-3), before their keys are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Authenticated,
    Roles,
    Internal,
}

const MODES: [(&str, Mode); 3] = [
    ("authenticated", Mode::Authenticated),
    ("roles", Mode::Roles),
    ("internal", Mode::Internal),
];

/// Reads the `access` of a dataset, whose roles are checked against `declared`; `summary`
/// says whether the dataset is a summary, the only kind `mode: internal` is valid on.
///
/// Returns the access when `problems` records nothing wrong with it.
pub(super) fn read_access(
    value: &Json,
    declared: &DeclaredRoles,
    summary: bool,
    problems: &mut Problems,
) -> Option<DatasetAccess> {
    let before = problems.len();
    let members = object(value, "`access`", &[MODE, ROLES, FIELDS], problems)?;
    let mode = read_mode(members, problems);
    let roles = read_mode_roles(members, mode, declared, problems);
    let fields = members
        .get(FIELDS)
        .and_then(|value| read_fields(value, mode, declared, problems));
    if mode == Some(Mode::Internal) && !summary {
        problems.push((
            ExtensionCode::InternalAccess,
            "`mode: internal` is valid only on a summary".to_owned(),
        ));
    }
    let mode = match mode? {
        Mode::Authenticated => AccessMode::Authenticated,
        Mode::Roles => AccessMode::Roles(roles?),
        Mode::Internal => AccessMode::Internal,
    };
    (problems.len() == before).then(|| DatasetAccess {
        mode,
        fields: fields.unwrap_or_default(),
    })
}

/// The `mode`: `E0C-41` when it is missing or outside SCY-3.
fn read_mode(members: &Map<String, Json>, problems: &mut Problems) -> Option<Mode> {
    let Some(value) = members.get(MODE) else {
        problems.push((
            ExtensionCode::AccessMode,
            "`access` requires `mode`".to_owned(),
        ));
        return None;
    };
    let found = MODES
        .iter()
        .find(|(name, _)| value.as_str() == Some(name))
        .map(|(_, mode)| *mode);
    if found.is_none() {
        problems.push((
            ExtensionCode::AccessMode,
            format!(
                "the `mode` of `access` is {value}, not `authenticated`, `roles` or `internal`"
            ),
        ));
    }
    found
}

/// `access.roles`: required with `mode: roles` and not empty (`E0C-42`), allowed only there
/// (`E0C-43`), without repeats (`E0C-03`), and declared (`E0C-47`). Without a valid mode only
/// its value is read.
fn read_mode_roles(
    members: &Map<String, Json>,
    mode: Option<Mode>,
    declared: &DeclaredRoles,
    problems: &mut Problems,
) -> Option<Vec<Role>> {
    let Some(value) = members.get(ROLES) else {
        if mode == Some(Mode::Roles) {
            problems.push((
                ExtensionCode::AccessRoles,
                "`mode: roles` requires `roles`".to_owned(),
            ));
        }
        return None;
    };
    let found = roles(value, "`access.roles`", ListRule::WithoutRepeats, problems)?;
    match mode {
        Some(Mode::Roles) if found.is_empty() => problems.push((
            ExtensionCode::AccessRoles,
            "`mode: roles` requires a non-empty `roles` list".to_owned(),
        )),
        Some(Mode::Authenticated | Mode::Internal) => problems.push((
            ExtensionCode::AccessKeys,
            "`roles` is allowed only with `mode: roles`".to_owned(),
        )),
        Some(Mode::Roles) | None => {}
    }
    declared.check("`access.roles`", &found, problems);
    Some(found)
}

/// `access.fields`: allowed with `authenticated` and `roles` (`E0C-43`); a list of objects
/// with exactly a non-empty `names` list and a non-empty `roles` list of declared roles, both
/// without repeats.
fn read_fields(
    value: &Json,
    mode: Option<Mode>,
    declared: &DeclaredRoles,
    problems: &mut Problems,
) -> Option<Vec<FieldRestriction>> {
    if mode == Some(Mode::Internal) {
        problems.push((
            ExtensionCode::AccessKeys,
            "`fields` is not allowed with `mode: internal`".to_owned(),
        ));
    }
    let Json::Array(items) = value else {
        problems.push(wrong_value("`access.fields`", value, "a list of objects"));
        return None;
    };
    let mut restrictions = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let what = format!("item {index} of `access.fields`");
        let Some(members) = object(item, &what, &[NAMES, ROLES], problems) else {
            continue;
        };
        let names = required(members, &what, NAMES, problems).and_then(|value| {
            let names = strings(value, &format!("the `names` of {what}"), problems)?;
            if names.is_empty() || has_repeats(&names) {
                problems.push(wrong_value(
                    &format!("the `names` of {what}"),
                    value,
                    "a non-empty list of strings without repeats",
                ));
            }
            Some(names)
        });
        let roles = required(members, &what, ROLES, problems).and_then(|value| {
            roles(
                value,
                &format!("the `roles` of {what}"),
                ListRule::NonEmptyWithoutRepeats,
                problems,
            )
        });
        if let Some(roles) = &roles {
            declared.check(&format!("the `roles` of {what}"), roles, problems);
        }
        if let (Some(names), Some(roles)) = (names, roles) {
            restrictions.push(FieldRestriction {
                names: names.into_iter().map(str::to_owned).collect(),
                roles,
            });
        }
    }
    Some(restrictions)
}
