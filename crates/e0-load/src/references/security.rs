//! `E0C-49`: every field `access.fields` names is a field of the dataset, and every role it
//! lists has access to the dataset (SCY-5.1.4).
//!
//! A field name is a declared name (E0D-1.4) compared under E0D-1.3, and a role has access
//! when it is listed in `access.roles` or the mode is `authenticated`, as the `security` crate
//! decides (S5).

use std::collections::HashMap;

use crate::extensions::security::DeclaredAccess;
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions, Problems, report_at};
use crate::names::Name;
use crate::outline::{DatasetOutline, outline};
use crate::pointer::JsonPointer;
use crate::structure::Structure;

/// Resolves the fields and roles of every `access.fields` of `extensions`, in the semantic
/// model that `structure` selected.
///
/// # Errors
///
/// Returns one `E0C-49` diagnostic per entry whose `access.fields` names a missing field or a
/// role without access to the dataset, at the entry's `data` member (E0C-2.6), in document
/// order. `E0C-49` rejects the model (E0-4).
pub fn resolve_security_references(
    structure: &Structure<'_>,
    extensions: &Extensions,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let datasets: HashMap<&JsonPointer, &DatasetOutline<'_>> = model
        .datasets
        .iter()
        .map(|dataset| (&dataset.pointer, dataset))
        .collect();
    let mut errors = Vec::new();
    for declared in &extensions.security.accesses {
        let Some(dataset) = datasets.get(&declared.dataset) else {
            continue;
        };
        let problems = field_problems(dataset, declared);
        errors.extend(report_at(&declared.data, declared.location, problems));
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The missing fields and the roles without access of the `access.fields` of `declared`.
fn field_problems(dataset: &DatasetOutline<'_>, declared: &DeclaredAccess) -> Problems {
    let mut problems = Problems::new();
    for (index, restriction) in declared.access.fields.iter().enumerate() {
        for name in &restriction.names {
            if dataset.field(&Name::declared(name)).is_none() {
                problems.push((
                    ExtensionCode::AccessFieldReference,
                    format!("item {index} of `access.fields` names `{name}`, which is not a field of the dataset"),
                ));
            }
        }
        for role in &restriction.roles {
            if !declared.access.grants(role) {
                problems.push((
                    ExtensionCode::AccessFieldReference,
                    format!("item {index} of `access.fields` lists the role `{role}`, which has no access to the dataset"),
                ));
            }
        }
    }
    problems
}
