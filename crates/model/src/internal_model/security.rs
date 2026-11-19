//! The security of the model: roles, attributes, dataset access and row policies (E0H-2
//! "Security", SCY-2 to SCY-4).

use super::common::{NormalizedName, SourceLocation};
use crate::expression::Expression;

/// A role of the security context, compared as an exact string (SCY-2).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoleName(String);

impl RoleName {
    /// The role `name`.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self(name.to_owned())
    }

    /// The role as written.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The `security_context` of the model (SCY-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityContext {
    /// The declared roles, ordered by name.
    pub roles: Vec<RoleName>,
    /// The declared attributes, ordered by name.
    pub attributes: Vec<SecurityAttribute>,
}

/// One declared attribute of the security context (SCY-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityAttribute {
    /// The name, as written.
    pub name: String,
    /// The type (SCY-2.2).
    pub attribute_type: AttributeType,
}

/// The type of an attribute (SCY-2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttributeType {
    /// One value.
    Scalar(ScalarType),
    /// `list<…>`: a list of values.
    List(ScalarType),
}

/// The type of one attribute value (SCY-2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScalarType {
    /// `string`.
    Text,
    /// `integer`.
    Integer,
    /// `decimal(p,s)`.
    Decimal {
        /// Total digits.
        precision: u8,
        /// Digits after the point.
        scale: u8,
    },
    /// `date`.
    Date,
    /// `boolean`.
    Boolean,
}

/// The access of one dataset (SCY-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetAccess {
    /// The mode, with its roles.
    pub mode: AccessMode,
    /// The field restrictions (SCY-3.1), in the order written.
    pub fields: Vec<FieldRestriction>,
}

/// The `mode` of a dataset's access (SCY-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessMode {
    /// `authenticated`.
    Authenticated,
    /// `roles`, ordered by name.
    Roles(Vec<RoleName>),
    /// `internal`: only on summaries.
    Internal,
}

/// One item of `access.fields` (SCY-3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRestriction {
    /// The restricted fields, ordered by name.
    pub fields: Vec<NormalizedName>,
    /// The roles they are restricted to, ordered by name.
    pub roles: Vec<RoleName>,
}

/// One row policy of a dataset (SCY-4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowPolicy {
    /// The name, as written: unique in the model.
    pub name: String,
    /// The normalized tree of `condition`, which E0.f parsed and typed (SCY-5.1.6).
    pub condition: Option<Expression>,
    /// The roles the policy grants rows to, ordered by name; `None` when absent.
    pub roles: Option<Vec<RoleName>>,
    /// The roles the policy does not apply to, ordered by name; `None` when absent.
    pub exempt_roles: Option<Vec<RoleName>>,
    /// Where the entry of ours that declares it is written.
    pub location: SourceLocation,
}
