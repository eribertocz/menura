//! The admitted security context: what the compiler reads after SEC-3.8.

use std::collections::BTreeMap;

use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_contracts::{ContractKind, Sha256Hash};

use super::fingerprint::fingerprint;
use super::value::AdmittedValue;
use crate::role::Role;

/// A security context admitted against the model's declarations (SEC-3.2, SEC-3.2a, SEC-3.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedSecurityContext {
    pub(crate) subject: String,
    pub(crate) roles: Vec<Role>,
    pub(crate) attributes: BTreeMap<String, AdmittedValue>,
    pub(crate) ignored_roles: Vec<Role>,
    pub(crate) ignored_attributes: Vec<String>,
    pub(crate) former_name: bool,
}

impl AdmittedSecurityContext {
    /// The opaque user identifier, for the audit record only: never in SQL or authorization
    /// (SEC-2).
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// The roles the user holds that the model declares, in CON-5 order (SEC-3.2).
    #[must_use]
    pub fn roles(&self) -> &[Role] {
        &self.roles
    }

    /// The attributes the model declares, by name, with values typed as declared (SEC-3.3).
    /// A declared attribute the context does not provide is absent.
    #[must_use]
    pub fn attributes(&self) -> &BTreeMap<String, AdmittedValue> {
        &self.attributes
    }

    /// The roles the model does not declare, in CON-5 order: they grant nothing (SEC-3.2).
    #[must_use]
    pub fn ignored_roles(&self) -> &[Role] {
        &self.ignored_roles
    }

    /// The names of the attributes the model does not declare, in CON-5 order (SEC-3.2a).
    #[must_use]
    pub fn ignored_attributes(&self) -> &[String] {
        &self.ignored_attributes
    }

    /// The context fingerprint (SEC-3.6), over the declared roles and attributes only.
    #[must_use]
    pub fn fingerprint(&self) -> Sha256Hash {
        fingerprint(&self.roles, &self.attributes)
    }

    /// The `WQRY-01` warning of a context read through the former contract name, located at
    /// `/contract` of the security context (SEC-3.9, CON-0.5); `None` for the current name.
    #[must_use]
    pub fn former_name_warning(&self) -> Option<Diagnostic> {
        self.former_name.then(|| {
            let kind = ContractKind::SecurityContext;
            Diagnostic {
                code: "WQRY-01".to_owned(),
                message: format!(
                    "`{}` is the former contract name; it is read as `{kind}` until 1.0.0, so \
                     write `{kind}` instead",
                    kind.former_contract_name()
                ),
                location: Some(DiagnosticLocation {
                    document: InputDocument::SecurityContext,
                    pointer: "/contract".to_owned(),
                    line: None,
                    column: None,
                }),
            }
        })
    }
}
