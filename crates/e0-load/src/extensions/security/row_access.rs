//! Reading one `row_access` value (SCY-4, SCY-5.1.5).

use menura_security::Role;
use serde_json::{Map, Value as Json};

use super::values::{DeclaredRoles, ListRule, object, required, roles, text, wrong_value};
use crate::extensions::diagnostic::ExtensionCode;
use crate::extensions::problems::Problems;
use crate::names::Name;

const NAME: &str = "name";
const CONDITION: &str = "condition";
const ROLES: &str = "roles";
const EXEMPT_ROLES: &str = "exempt_roles";

/// The keys of a row policy (SCY-4).
const POLICY_KEYS: [&str; 4] = [NAME, CONDITION, ROLES, EXEMPT_ROLES];

/// One row policy whose shape and roles are valid, before it is placed on its dataset.
#[derive(Debug)]
pub(super) struct PolicyItem {
    pub(super) name: String,
    pub(super) condition: String,
    pub(super) roles: Option<Vec<Role>>,
    pub(super) exempt_roles: Option<Vec<Role>>,
}

/// Reads the `row_access` list of a dataset. `names` holds the policy names seen so far in the
/// model, compared under E0D-1.3: a repeated one is `E0C-45` (SCY-5.1.1, SCY-5.1.5).
pub(super) fn read_row_access(
    value: &Json,
    declared: &DeclaredRoles,
    names: &mut Vec<Name>,
    problems: &mut Problems,
) -> Vec<PolicyItem> {
    let Json::Array(items) = value else {
        problems.push(wrong_value("`row_access`", value, "a list of objects"));
        return Vec::new();
    };
    let mut policies = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let before = problems.len();
        let what = format!("item {index} of `row_access`");
        let Some(members) = object(item, &what, &POLICY_KEYS, problems) else {
            continue;
        };
        let name = required(members, &what, NAME, problems)
            .and_then(|value| text(value, &format!("the `name` of {what}"), problems));
        if let Some(name) = name {
            let normalized = Name::declared(name);
            if names.contains(&normalized) {
                problems.push((
                    ExtensionCode::DuplicatePolicy,
                    format!("the row policy name `{name}` ({what}) is already used in the model"),
                ));
            } else {
                names.push(normalized);
            }
        }
        let condition = required(members, &what, CONDITION, problems)
            .and_then(|value| text(value, &format!("the `condition` of {what}"), problems));
        let roles = optional_roles(members, ROLES, &what, declared, problems);
        let exempt_roles = optional_roles(members, EXEMPT_ROLES, &what, declared, problems);
        if let (Some(name), Some(condition), true) = (name, condition, problems.len() == before) {
            policies.push(PolicyItem {
                name: name.to_owned(),
                condition: condition.to_owned(),
                roles,
                exempt_roles,
            });
        }
    }
    policies
}

/// The optional role list `key` of a policy: a non-empty list of declared roles without
/// repeats.
fn optional_roles(
    members: &Map<String, Json>,
    key: &str,
    what: &str,
    declared: &DeclaredRoles,
    problems: &mut Problems,
) -> Option<Vec<Role>> {
    let value = members.get(key)?;
    let what = format!("the `{key}` of {what}");
    let found = roles(value, &what, ListRule::NonEmptyWithoutRepeats, problems)?;
    declared.check(&what, &found, problems);
    Some(found)
}
