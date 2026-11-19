//! The access every dataset is read with (SCY-3.0, SCY-5.1.7, DEV-4): `W0C-40` and `W0C-41`,
//! and the load option `grant_authenticated_to_all`.

use semantic_compiler_contracts::types::audit_record::OssieFormat;
use semantic_compiler_security::{AccessMode, DatasetAccess, Role};

use super::fixture::{MODEL, NOTES, ORDERS, SEMANTIC_MODEL, conformance_model};
use super::resolution::{ResolvedAccess, resolve_access};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::structure::validate_structure;

fn roles(names: &[&str]) -> Vec<Role> {
    names.iter().map(|name| Role::new(name)).collect()
}

/// The access of every dataset of `model` and the codes of the warnings about it.
fn resolved(
    model: &str,
    grant_authenticated_to_all: bool,
) -> (ResolvedAccess, Vec<(&'static str, String)>) {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure =
        validate_structure(&document, OssieFormat::Release, None).expect("the structure is valid");
    let extensions = validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
        .expect("the extensions are accepted");
    let resolution = resolve_access(
        &document,
        &structure.semantic_model,
        &extensions.security,
        grant_authenticated_to_all,
    );
    let warnings = resolution
        .warnings
        .iter()
        .map(|warning| (warning.code.as_str(), warning.pointer.to_string()))
        .collect();
    (resolution, warnings)
}

#[test]
fn scy_3_0_scy_5_1_7_w0c_41_a_dataset_without_access_is_inaccessible() {
    let (resolution, warnings) = resolved(MODEL, false);
    assert_eq!(warnings, [("W0C-41", NOTES.to_owned())]);
    assert_eq!(resolution.datasets.len(), 2);
    assert_eq!(resolution.datasets[0].dataset.to_string(), ORDERS);
    assert!(resolution.datasets[0].access.is_some());
    assert_eq!(resolution.datasets[1].dataset.to_string(), NOTES);
    assert_eq!(resolution.datasets[1].access, None);
    assert!(resolution.warnings[0].message.contains("`notes`"));
}

#[test]
fn scy_5_1_7_dev_4_grant_authenticated_to_all_opens_datasets_without_access() {
    let (resolution, warnings) = resolved(MODEL, true);
    assert!(warnings.is_empty());
    assert_eq!(
        resolution.datasets[1].access,
        Some(DatasetAccess::granted_to_authenticated())
    );
    assert_eq!(
        resolution.datasets[0]
            .access
            .as_ref()
            .map(|access| &access.mode),
        Some(&AccessMode::Roles(roles(&["analyst", "admin"])))
    );
}

#[test]
fn scy_2_1_scy_5_1_7_w0c_40_a_model_without_security_context() {
    let model = conformance_model("e0-security-no-context-001");
    let (resolution, warnings) = resolved(model, false);
    assert_eq!(warnings, [("W0C-40", SEMANTIC_MODEL.to_owned())]);
    assert!(
        resolution
            .datasets
            .iter()
            .all(|dataset| dataset.access.is_some())
    );
    let (_, warnings) = resolved(model, true);
    assert_eq!(warnings, [("W0C-40", SEMANTIC_MODEL.to_owned())]);
}
