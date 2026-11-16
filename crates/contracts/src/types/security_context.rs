//! The security context document (SEC-2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::contract::ContractVersion;

/// The `contract` value `semantic_compiler.security_context`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityContextContract {
    /// The only value.
    #[serde(rename = "semantic_compiler.security_context")]
    SecurityContext,
}

/// Who is asking (SEC-2). The host has already authenticated the subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityContextDocument {
    /// Always `semantic_compiler.security_context`.
    pub contract: SecurityContextContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// Opaque user identifier, used only in the audit record.
    pub subject: String,
    /// Roles granted to the user, without duplicates.
    pub roles: Vec<String>,
    /// Attribute values row policies read.
    pub attributes: BTreeMap<String, AttributeValue>,
}

/// An attribute value: a scalar or a list of scalars.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    /// One scalar.
    Scalar(AttributeScalar),
    /// A list of scalars.
    List(Vec<AttributeScalar>),
}

/// A scalar attribute value, encoded as CON-4 requires.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AttributeScalar {
    /// Text, or a canonical string encoding (decimal, date, large integer, …).
    Text(String),
    /// An integer within ±(2^53 − 1).
    Integer(i64),
    /// A boolean.
    Boolean(bool),
}
