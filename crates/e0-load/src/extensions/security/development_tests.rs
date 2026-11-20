//! The security extension in the development format (MDL-2.4, E0C-2.8): the same rules read
//! from `nonportable_extensions`.

use menura_contracts::types::audit_record::OssieFormat;
use menura_security::{AccessMode, Role};

use super::fixture::conformance_model;
use crate::extensions::output::{Extensions, ExtensionsFailure};
use crate::extensions::{DEFAULT_NAMESPACE, validate_extensions};
use crate::read::read_document;
use crate::structure::validate_structure;

fn validate(model: &str) -> Result<Extensions, ExtensionsFailure> {
    let document = read_document(model.as_bytes()).expect("the document is read");
    let structure = validate_structure(&document, OssieFormat::Development, None)
        .expect("the structure is valid");
    validate_extensions(&document, &structure.semantic_model, DEFAULT_NAMESPACE)
}

#[test]
fn mdl_2_4_scy_5_1_3_scy_5_1_5_the_development_model_is_read() {
    let security = validate(conformance_model("e0-security-development"))
        .expect("the extensions are accepted")
        .security;
    assert_eq!(security.model.to_string(), "");
    let context = security.context.expect("a security context");
    assert_eq!(context.roles, [Role::new("admin"), Role::new("analyst")]);
    assert_eq!(security.accesses.len(), 2);
    assert_eq!(security.accesses[0].dataset.to_string(), "/datasets/0");
    assert_eq!(security.accesses[1].access.mode, AccessMode::Authenticated);
    assert_eq!(security.policies.len(), 1);
    assert_eq!(
        security.policies[0].condition,
        "region IN (USER_ATTRIBUTE('regions'))"
    );
}

#[test]
fn e0c_2_8_scy_5_1_3_e0c_41_development_access_without_mode() {
    let failure = validate(conformance_model("e0-security-development-mode-001"))
        .expect_err("the extensions are rejected");
    let found: Vec<(&str, String)> = failure
        .errors
        .iter()
        .map(|diagnostic| (diagnostic.code.as_str(), diagnostic.pointer.to_string()))
        .collect();
    assert_eq!(
        found,
        [(
            "E0C-41",
            "/datasets/0/nonportable_extensions/0/data".to_owned()
        )]
    );
}
