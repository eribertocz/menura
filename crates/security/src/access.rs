//! The access of a dataset (SCY-3), as data types, and who it grants.

use crate::role::Role;

/// The `mode` of a dataset's `access` (SCY-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessMode {
    /// `authenticated`: any user with a security context.
    Authenticated,
    /// `roles`: users holding at least one of these roles, as listed.
    Roles(Vec<Role>),
    /// `internal`: no consumer; only on summaries.
    Internal,
}

/// One item of `access.fields`: named fields of the dataset restricted to some roles
/// (SCY-3.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldRestriction {
    /// The field names, as written: declared names of fields of the dataset (SCY-5.1.1).
    pub names: Vec<String>,
    /// The roles the fields are restricted to.
    pub roles: Vec<Role>,
}

/// The access of one dataset (SCY-3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetAccess {
    /// The mode, with its roles.
    pub mode: AccessMode,
    /// The field restrictions, in the order written.
    pub fields: Vec<FieldRestriction>,
}

impl DatasetAccess {
    /// The access a dataset without `access` is read with under the load option
    /// `grant_authenticated_to_all: true`: `mode: authenticated` without `fields` (SCY-5.1.7,
    /// DEV-4).
    #[must_use]
    pub fn granted_to_authenticated() -> Self {
        Self {
            mode: AccessMode::Authenticated,
            fields: Vec::new(),
        }
    }

    /// Whether `role` has access to the dataset: it is listed in `access.roles`, or the mode is
    /// `authenticated` (SCY-5.1.4). No role has access to an internal dataset.
    ///
    /// ```
    /// use semantic_compiler_security::{AccessMode, DatasetAccess, Role};
    ///
    /// let access = DatasetAccess { mode: AccessMode::Roles(vec![Role::new("analyst")]), fields: Vec::new() };
    /// assert!(access.grants(&Role::new("analyst")));
    /// assert!(!access.grants(&Role::new("admin")));
    /// ```
    #[must_use]
    pub fn grants(&self, role: &Role) -> bool {
        match &self.mode {
            AccessMode::Authenticated => true,
            AccessMode::Roles(roles) => roles.contains(role),
            AccessMode::Internal => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scy_5_1_4_authenticated_grants_every_role_and_internal_none() {
        let analyst = Role::new("analyst");
        assert!(DatasetAccess::granted_to_authenticated().grants(&analyst));
        let internal = DatasetAccess {
            mode: AccessMode::Internal,
            fields: Vec::new(),
        };
        assert!(!internal.grants(&analyst));
    }

    #[test]
    fn scy_5_1_1_roles_are_compared_as_exact_strings() {
        let access = DatasetAccess {
            mode: AccessMode::Roles(vec![Role::new("Analyst")]),
            fields: Vec::new(),
        };
        assert!(!access.grants(&Role::new("analyst")));
        assert!(access.grants(&Role::new("Analyst")));
    }
}
