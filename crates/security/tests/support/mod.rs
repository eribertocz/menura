//! Shared builders for the security context tests.

#![allow(
    dead_code,
    reason = "each test file uses a different part of the shared support module"
)]

use menura_model::internal_model::{
    AttributeType, RoleName, ScalarType, SecurityAttribute, SecurityContext,
};

/// The declarations of a model: `roles` and `(name, type)` attributes, as E0.h orders them.
pub fn declarations(roles: &[&str], attributes: &[(&str, AttributeType)]) -> SecurityContext {
    let mut roles: Vec<RoleName> = roles.iter().map(|role| RoleName::new(role)).collect();
    roles.sort();
    let mut attributes: Vec<SecurityAttribute> = attributes
        .iter()
        .map(|(name, attribute_type)| SecurityAttribute {
            name: (*name).to_owned(),
            attribute_type: *attribute_type,
        })
        .collect();
    attributes.sort_by(|left, right| left.name.cmp(&right.name));
    SecurityContext { roles, attributes }
}

/// A scalar attribute type.
pub fn scalar(item: ScalarType) -> AttributeType {
    AttributeType::Scalar(item)
}

/// A list attribute type.
pub fn list(item: ScalarType) -> AttributeType {
    AttributeType::List(item)
}

/// `decimal(precision,scale)`.
pub fn decimal(precision: u8, scale: u8) -> ScalarType {
    ScalarType::Decimal { precision, scale }
}

/// A security context document with `roles` and `attributes` (JSON texts), named `contract`.
pub fn document_named(contract: &str, subject: &str, roles: &str, attributes: &str) -> String {
    format!(
        r#"{{"contract": "{contract}", "contract_version": "1.1.0", "subject": "{subject}",
            "roles": {roles}, "attributes": {attributes}}}"#
    )
}

/// A security context document named `menura.security_context`.
pub fn document(roles: &str, attributes: &str) -> String {
    document_named("menura.security_context", "user-1", roles, attributes)
}
