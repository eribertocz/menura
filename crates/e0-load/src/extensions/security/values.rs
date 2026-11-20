//! Reading the values the security extension shares: objects with known keys, lists of
//! strings and role lists (SCY-5.1.2, SCY-5.1.3, SCY-5.1.5). Objects with known keys,
//! required keys, strings and the `E0C-03` problem are read the same way by the summary
//! extension (SUM-6.1.1).

use std::collections::BTreeSet;

use menura_security::Role;
use serde_json::{Map, Value as Json};

use crate::extensions::diagnostic::ExtensionCode;
use crate::extensions::problems::Problems;

/// The roles the model declares, which the roles of `access` and `row_access` are checked
/// against (`E0C-47`).
#[derive(Debug, Clone)]
pub(super) enum DeclaredRoles {
    /// The roles of `security_context`; none when the model has no `security_context`
    /// (SCY-2.1).
    Known(BTreeSet<Role>),
    /// `security_context.roles` is not a list of strings: it is `E0C-03`, and no role is
    /// reported as undeclared.
    Unknown,
}

impl DeclaredRoles {
    /// Records `E0C-47` for every role of `roles` the model does not declare.
    pub(super) fn check(&self, what: &str, roles: &[Role], problems: &mut Problems) {
        let Self::Known(declared) = self else {
            return;
        };
        for role in roles {
            if !declared.contains(role) {
                problems.push((
                    ExtensionCode::UndeclaredRole,
                    format!(
                        "{what} lists the role `{role}`, which security_context does not declare"
                    ),
                ));
            }
        }
    }
}

/// The members of `value`, recording `E0C-03` when it is not an object, and `E0C-02` for each
/// key outside `keys` (E0C-2.4).
pub(in crate::extensions) fn object<'json>(
    value: &'json Json,
    what: &str,
    keys: &[&str],
    problems: &mut Problems,
) -> Option<&'json Map<String, Json>> {
    let Json::Object(members) = value else {
        problems.push(wrong_value(what, value, "an object"));
        return None;
    };
    for key in members.keys() {
        if !keys.contains(&key.as_str()) {
            let message = format!("{what} has no key `{key}`; its keys are {}", listed(keys));
            problems.push((ExtensionCode::UnknownKey, message));
        }
    }
    Some(members)
}

/// The value of the required `key` of `members`, recording `E0C-03` when it is absent.
pub(in crate::extensions) fn required<'json>(
    members: &'json Map<String, Json>,
    what: &str,
    key: &str,
    problems: &mut Problems,
) -> Option<&'json Json> {
    let value = members.get(key);
    if value.is_none() {
        problems.push((
            ExtensionCode::WrongValue,
            format!("{what} requires `{key}`"),
        ));
    }
    value
}

/// The text of `value`, recording `E0C-03` when it is not a string.
pub(in crate::extensions) fn text<'json>(
    value: &'json Json,
    what: &str,
    problems: &mut Problems,
) -> Option<&'json str> {
    let found = value.as_str();
    if found.is_none() {
        problems.push(wrong_value(what, value, "a string"));
    }
    found
}

/// The strings of `value`, recording `E0C-03` when it is not a list of strings.
pub(super) fn strings<'json>(
    value: &'json Json,
    what: &str,
    problems: &mut Problems,
) -> Option<Vec<&'json str>> {
    let found = value
        .as_array()
        .and_then(|items| items.iter().map(Json::as_str).collect::<Option<Vec<_>>>());
    if found.is_none() {
        problems.push(wrong_value(what, value, "a list of strings"));
    }
    found
}

/// The roles of `value`, recording `E0C-03` when it is not a list of strings; `rule` adds
/// what else the list must be.
pub(super) fn roles(
    value: &Json,
    what: &str,
    rule: ListRule,
    problems: &mut Problems,
) -> Option<Vec<Role>> {
    let names = strings(value, what, problems)?;
    let roles: Vec<Role> = names.iter().map(|name| Role::new(name)).collect();
    let expected = match rule {
        ListRule::Any => None,
        ListRule::WithoutRepeats => {
            has_repeats(&names).then_some("a list of strings without repeats")
        }
        ListRule::NonEmptyWithoutRepeats => (roles.is_empty() || has_repeats(&names))
            .then_some("a non-empty list of strings without repeats"),
    };
    if let Some(expected) = expected {
        problems.push(wrong_value(what, value, expected));
    }
    Some(roles)
}

/// What a list of strings must be besides a list of strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ListRule {
    /// Nothing more.
    Any,
    /// No string repeated; it may be empty.
    WithoutRepeats,
    /// Not empty, and no string repeated.
    NonEmptyWithoutRepeats,
}

/// Whether a string of `names` is repeated.
pub(super) fn has_repeats(names: &[&str]) -> bool {
    let mut seen = BTreeSet::new();
    names.iter().any(|name| !seen.insert(*name))
}

/// The `E0C-03` problem of `what`, whose value `value` is not `expected`.
pub(in crate::extensions) fn wrong_value(
    what: &str,
    value: &Json,
    expected: &str,
) -> (ExtensionCode, String) {
    (
        ExtensionCode::WrongValue,
        format!("{what} is {value}, not {expected}"),
    )
}

/// `keys` as a message lists them, such as `` `a` and `b` ``.
fn listed(keys: &[&str]) -> String {
    let quoted: Vec<String> = keys.iter().map(|key| format!("`{key}`")).collect();
    match quoted.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
        None => String::new(),
    }
}
