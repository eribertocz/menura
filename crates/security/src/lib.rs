//! Access and row-policy logic, the single home of security rules (S5).
//!
//! E0 reads the security extension with these types and rules: the attribute types of the
//! security context (SCY-2.2), the access of a dataset (SCY-3), the place a list attribute may
//! take in a row-policy condition (SCY-4.1), and the roles every field and metric is
//! restricted to (SCY-3.1, SCY-3.1a, SCY-5.1.8).
//!
//! The compiler admits the security context with [`admit_security_context`]: the order of
//! checks of SEC-3.8, the declared types of SEC-3.3, and the fingerprint of SEC-3.6.

#![forbid(unsafe_code)]

mod access;
mod attribute_type;
mod context;
mod list_attributes;
mod restrictions;
mod role;
mod walk;

pub use access::{AccessMode, DatasetAccess, FieldRestriction};
pub use attribute_type::{AttributeType, ScalarType};
pub use context::{
    AdmittedScalar, AdmittedSecurityContext, AdmittedValue, DateValue, DecimalValue,
    admit_security_context,
};
pub use list_attributes::misplaced_list_attributes;
pub use restrictions::{RestrictionInput, Restrictions, restrictions};
pub use role::Role;
