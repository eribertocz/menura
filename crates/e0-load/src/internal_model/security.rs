//! The security of the model in the form of the internal model (E0H-2 "Security"): roles,
//! attributes, dataset access and parsed row policies.

use menura_model::internal_model::{
    AccessMode, AttributeType, DatasetAccess, FieldRestriction, RoleName, RowPolicy, ScalarType,
    SecurityAttribute, SecurityContext,
};
use menura_security as security;

use super::located::{located, written};
use super::trees::parse;
use crate::expressions::ExpressionPlace;
use crate::extensions::security::{RowPolicy as DeclaredPolicy, SecurityContextDeclaration};

/// The model's form of the `security_context`.
pub(super) fn security_context(declared: &SecurityContextDeclaration) -> SecurityContext {
    let mut attributes: Vec<SecurityAttribute> = declared
        .attributes
        .iter()
        .map(|attribute| SecurityAttribute {
            name: attribute.name.clone(),
            attribute_type: attribute_type(attribute.attribute_type),
        })
        .collect();
    attributes.sort_by(|left, right| left.name.cmp(&right.name));
    SecurityContext {
        roles: roles(&declared.roles),
        attributes,
    }
}

fn attribute_type(declared: security::AttributeType) -> AttributeType {
    match declared {
        security::AttributeType::Scalar(item) => AttributeType::Scalar(scalar(item)),
        security::AttributeType::List(item) => AttributeType::List(scalar(item)),
    }
}

fn scalar(declared: security::ScalarType) -> ScalarType {
    match declared {
        security::ScalarType::Text => ScalarType::Text,
        security::ScalarType::Integer => ScalarType::Integer,
        security::ScalarType::Decimal { precision, scale } => {
            ScalarType::Decimal { precision, scale }
        }
        security::ScalarType::Date => ScalarType::Date,
        security::ScalarType::Boolean => ScalarType::Boolean,
    }
}

/// The roles, ordered by name, without repeats.
fn roles(declared: &[security::Role]) -> Vec<RoleName> {
    let mut roles: Vec<RoleName> = declared
        .iter()
        .map(|role| RoleName::new(role.as_str()))
        .collect();
    roles.sort();
    roles.dedup();
    roles
}

/// The model's form of a dataset access.
pub(super) fn dataset_access(declared: &security::DatasetAccess) -> DatasetAccess {
    DatasetAccess {
        mode: match &declared.mode {
            security::AccessMode::Authenticated => AccessMode::Authenticated,
            security::AccessMode::Roles(list) => AccessMode::Roles(roles(list)),
            security::AccessMode::Internal => AccessMode::Internal,
        },
        fields: declared
            .fields
            .iter()
            .map(|restriction| {
                let mut fields: Vec<_> =
                    restriction.names.iter().map(|name| written(name)).collect();
                fields.sort();
                FieldRestriction {
                    fields,
                    roles: roles(&restriction.roles),
                }
            })
            .collect(),
    }
}

/// The model's form of a row policy, with its condition parsed.
pub(super) fn row_policy(declared: &DeclaredPolicy) -> RowPolicy {
    RowPolicy {
        name: declared.name.clone(),
        condition: parse(&declared.condition, ExpressionPlace::Extension).ok(),
        roles: declared.roles.as_deref().map(roles),
        exempt_roles: declared.exempt_roles.as_deref().map(roles),
        location: located(&declared.data, declared.location),
    }
}
