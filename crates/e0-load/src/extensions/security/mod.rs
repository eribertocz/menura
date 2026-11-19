//! The security extension, in E0.c: the shape and keys of `security_context` (SCY-5.1.2), of
//! every `access` (SCY-5.1.3: `E0C-41` to `E0C-44`) and of every `row_access` (SCY-5.1.5:
//! `E0C-45`), with their undeclared roles (`E0C-47`), and the access of every dataset with the
//! warnings `W0C-40` and `W0C-41` (SCY-5.1.7).
//!
//! The fields and roles of `access.fields` are resolved in E0.d (`E0C-49`, SCY-5.1.4), and the
//! conditions of row policies are parsed and typed in E0.f (`E0C-46`, `E0C-48`, SCY-5.1.6),
//! each against the [`Security`] read here. The security rules themselves live in the
//! `security` crate (S5).

mod access;
mod context;
#[cfg(test)]
pub(crate) mod fixture;
mod model;
pub(crate) mod resolution;
mod row_access;
mod values;

pub(crate) use model::{DeclaredAccess, RowPolicy, Security, SecurityContextDeclaration};

use std::collections::BTreeSet;

use super::diagnostic::ExtensionDiagnostic;
use super::output::{ObjectKind, OwnEntry};
use super::problems::{Problems, report};
use crate::names::Name;
use crate::pointer::JsonPointer;
use access::read_access;
use context::read_security_context;
use row_access::read_row_access;
use values::DeclaredRoles;

/// The extension key of the security context (MDL-2).
const SECURITY_CONTEXT: &str = "security_context";

/// The extension key of the access of a dataset (MDL-2).
const ACCESS: &str = "access";

/// The extension key of row policies (MDL-2).
const ROW_ACCESS: &str = "row_access";

/// The extension key that makes a dataset a summary (SUM-2), the only kind of dataset
/// `mode: internal` is valid on (`E0C-44`).
const AGGREGATE: &str = "aggregate";

/// The security declarations of a model and the E0.c diagnostics about them.
#[derive(Debug, Default)]
pub(crate) struct SecurityValidation {
    /// Every declaration without problems.
    pub(crate) security: Security,
    /// At most one diagnostic per code and entry, in document order then by code.
    pub(crate) errors: Vec<ExtensionDiagnostic>,
}

/// Reads the security extension of every entry of `entries`, which are in document order, in
/// the semantic model at `model`.
pub(crate) fn validate_security(model: &JsonPointer, entries: &[OwnEntry]) -> SecurityValidation {
    let mut found = SecurityValidation {
        security: Security {
            model: model.clone(),
            ..Security::default()
        },
        errors: Vec::new(),
    };
    let declared = read_model_entries(entries, &mut found);
    let mut policy_names: Vec<Name> = Vec::new();
    for entry in entries
        .iter()
        .filter(|entry| entry.object == ObjectKind::Dataset)
    {
        read_dataset_entry(entry, &declared, &mut policy_names, &mut found);
    }
    found
}

/// Reads the `security_context` of the model entry of `entries`, and returns the roles it
/// declares: none without `security_context` (SCY-2.1).
fn read_model_entries(entries: &[OwnEntry], found: &mut SecurityValidation) -> DeclaredRoles {
    let mut declared = DeclaredRoles::Known(BTreeSet::new());
    for entry in entries {
        if let (ObjectKind::Model, Some(value)) =
            (entry.object, entry.extensions.get(SECURITY_CONTEXT))
        {
            let mut problems = Problems::new();
            let reading = read_security_context(value, &mut problems);
            found.security.context = reading.declaration;
            declared = reading.roles;
            found.errors.extend(report(entry, problems));
        }
    }
    declared
}

/// Reads the `access` and `row_access` of one dataset entry, comparing its policy names with
/// `policy_names`, the names seen so far in the model.
fn read_dataset_entry(
    entry: &OwnEntry,
    declared: &DeclaredRoles,
    policy_names: &mut Vec<Name>,
    found: &mut SecurityValidation,
) {
    let mut problems = Problems::new();
    if let Some(value) = entry.extensions.get(ACCESS) {
        let summary = entry.extensions.contains_key(AGGREGATE);
        if let Some(access) = read_access(value, declared, summary, &mut problems) {
            found.security.accesses.push(DeclaredAccess {
                dataset: entry.object_pointer.clone(),
                data: entry.data_pointer.clone(),
                location: entry.location,
                access,
            });
        }
    }
    if let Some(value) = entry.extensions.get(ROW_ACCESS) {
        for item in read_row_access(value, declared, policy_names, &mut problems) {
            found.security.policies.push(RowPolicy {
                dataset: entry.object_pointer.clone(),
                data: entry.data_pointer.clone(),
                location: entry.location,
                name: item.name,
                condition: item.condition,
                roles: item.roles,
                exempt_roles: item.exempt_roles,
            });
        }
    }
    found.errors.extend(report(entry, problems));
}

#[cfg(test)]
mod access_tests;
#[cfg(test)]
mod development_tests;
#[cfg(test)]
mod resolution_tests;
#[cfg(test)]
mod support;
#[cfg(test)]
mod tests;
