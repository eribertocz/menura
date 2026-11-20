//! Reading `security_context` (SCY-2, SCY-5.1.2).

use std::collections::BTreeSet;

use menura_security::{AttributeType, Role};
use serde_json::Value as Json;

use super::model::{Attribute, SecurityContextDeclaration};
use super::values::{
    DeclaredRoles, ListRule, has_repeats, object, required, roles, text, wrong_value,
};
use crate::extensions::diagnostic::ExtensionCode;
use crate::extensions::problems::Problems;

const ROLES: &str = "roles";
const ATTRIBUTES: &str = "attributes";
const NAME: &str = "name";
const TYPE: &str = "type";

/// What reading `security_context` found.
pub(super) struct ContextReading {
    /// The declaration, when it has no problem.
    pub(super) declaration: Option<SecurityContextDeclaration>,
    /// The roles it declares, which `access` and `row_access` are checked against.
    pub(super) roles: DeclaredRoles,
}

/// Reads one `security_context` value: an object with exactly `roles` and `attributes`
/// (SCY-5.1.2).
pub(super) fn read_security_context(value: &Json, problems: &mut Problems) -> ContextReading {
    let before = problems.len();
    let unknown_roles = ContextReading {
        declaration: None,
        roles: DeclaredRoles::Unknown,
    };
    let Some(members) = object(value, "`security_context`", &[ROLES, ATTRIBUTES], problems) else {
        return unknown_roles;
    };
    let roles = required(members, "`security_context`", ROLES, problems)
        .and_then(|value| read_roles(value, problems));
    let attributes = required(members, "`security_context`", ATTRIBUTES, problems)
        .and_then(|value| read_attributes(value, problems));
    let Some(roles) = roles else {
        return unknown_roles;
    };
    let declared = DeclaredRoles::Known(roles.iter().cloned().collect());
    let declaration = attributes
        .filter(|_| problems.len() == before)
        .map(|attributes| SecurityContextDeclaration { roles, attributes });
    ContextReading {
        declaration,
        roles: declared,
    }
}

/// `roles`: a list of non-empty strings without repeats. A list of strings with an empty or
/// repeated string is `E0C-03`, but its roles are still declared.
fn read_roles(value: &Json, problems: &mut Problems) -> Option<Vec<Role>> {
    let found = roles(value, "`security_context.roles`", ListRule::Any, problems)?;
    let names: Vec<&str> = found.iter().map(Role::as_str).collect();
    if names.contains(&"") || has_repeats(&names) {
        problems.push(wrong_value(
            "`security_context.roles`",
            value,
            "a list of non-empty strings without repeats",
        ));
    }
    Some(found)
}

/// `attributes`: a list of objects with exactly `name` and `type`, with no name repeated.
fn read_attributes(value: &Json, problems: &mut Problems) -> Option<Vec<Attribute>> {
    let Json::Array(items) = value else {
        problems.push(wrong_value(
            "`security_context.attributes`",
            value,
            "a list of objects",
        ));
        return None;
    };
    let mut attributes = Vec::new();
    let mut names = BTreeSet::new();
    let mut complete = true;
    for (index, item) in items.iter().enumerate() {
        match read_attribute(index, item, problems) {
            Some(attribute) => {
                if !names.insert(attribute.name.clone()) {
                    problems.push((
                        ExtensionCode::WrongValue,
                        format!(
                            "the attribute name `{}` is repeated in `security_context.attributes`",
                            attribute.name
                        ),
                    ));
                }
                attributes.push(attribute);
            }
            None => complete = false,
        }
    }
    complete.then_some(attributes)
}

/// The attribute at `index`: `name`, a non-empty string, and `type`, written as in SCY-2.2.
fn read_attribute(index: usize, item: &Json, problems: &mut Problems) -> Option<Attribute> {
    let what = format!("attribute {index} of `security_context.attributes`");
    let members = object(item, &what, &[NAME, TYPE], problems)?;
    let name = required(members, &what, NAME, problems).and_then(|value| {
        let name = text(value, &format!("the `name` of {what}"), problems)?;
        if name.is_empty() {
            problems.push(wrong_value(
                &format!("the `name` of {what}"),
                value,
                "a non-empty string",
            ));
            return None;
        }
        Some(name)
    });
    let attribute_type = required(members, &what, TYPE, problems).and_then(|value| {
        let written = text(value, &format!("the `type` of {what}"), problems)?;
        let parsed = AttributeType::parse(written);
        if parsed.is_none() {
            problems.push(wrong_value(
                &format!("the `type` of {what}"),
                value,
                "`text`, `integer`, `decimal(p,s)` with 1 ≤ p ≤ 38 and 0 ≤ s ≤ p, `date`, `boolean`, or `list<…>` of one of them",
            ));
        }
        parsed
    });
    Some(Attribute {
        name: name?.to_owned(),
        attribute_type: attribute_type?,
    })
}
