//! Admission of the security context against the model's declarations (SEC-3), and its
//! fingerprint (SEC-3.6).

mod admitted;
mod declared_type;
mod fingerprint;
mod value;

use std::collections::{BTreeMap, BTreeSet};

use menura_contracts::types::compiled_plan::InputDocument;
use menura_contracts::types::security_context::{SecurityContextContract, SecurityContextDocument};
use menura_contracts::{AdmissionCode, AdmissionError, IncomingContract, compare_names, decode};
use menura_model::internal_model::{AttributeType, RoleName, SecurityContext};

pub use admitted::AdmittedSecurityContext;
pub use value::{AdmittedScalar, AdmittedValue, DateValue, DecimalValue};

use crate::role::Role;
use declared_type::{Mismatch, check_value};

/// Admits a security context `text` against `declarations`, the model's `security_context`
/// (`None` when the model has none, and so declares no roles or attributes: SCY-2.1).
///
/// Undeclared roles and attributes are kept apart as ignored (SEC-3.2, SEC-3.2a); undeclared
/// attribute values are not checked against any type.
///
/// # Errors
///
/// The first failure, in the order of SEC-3.8 (E2-5 row 0a), located in the security context:
/// `ESEC-01` at the empty pointer, with line and column, when the text is not JSON; `ESEC-03`
/// at `/contract` or `/contract_version` when the contract or its version is not supported;
/// `ESEC-01` at the first violation of the JSON Schema; then `ESEC-01` at
/// `/attributes/<name>` or `/attributes/<name>/<index>` for the first declared attribute, in
/// CON-5 order of names, whose value does not match its declared type (SEC-3.3).
///
/// # Example
///
/// ```
/// use menura_contracts::AdmissionCode;
/// use menura_security::admit_security_context;
///
/// let text = r#"{"contract": "menura.security_context", "contract_version": "1.1.0",
///     "subject": "user-7f3a", "roles": ["analyst"], "attributes": {}}"#;
/// let context = admit_security_context(text, None).unwrap();
/// assert_eq!(context.ignored_roles()[0].as_str(), "analyst");
///
/// let error = admit_security_context("{}", None).unwrap_err();
/// assert_eq!(error.code, AdmissionCode::Esec03);
/// ```
pub fn admit_security_context(
    text: &str,
    declarations: Option<&SecurityContext>,
) -> Result<AdmittedSecurityContext, AdmissionError> {
    let contract = IncomingContract::SecurityContext;
    let json = contract.parse_json(text)?;
    contract.check_contract(&json)?;
    contract.validate(&json)?;
    let document: SecurityContextDocument = decode(contract, &json)?;
    let declared_roles: BTreeSet<&str> = declarations
        .iter()
        .flat_map(|declared| declared.roles.iter().map(RoleName::as_str))
        .collect();
    let declared_attributes: BTreeMap<&str, AttributeType> = declarations
        .iter()
        .flat_map(|declared| declared.attributes.iter())
        .map(|attribute| (attribute.name.as_str(), attribute.attribute_type))
        .collect();
    let (roles, ignored_roles) = split_roles(&document.roles, |role| declared_roles.contains(role));
    let mut admitted = AdmittedSecurityContext {
        subject: document.subject,
        roles,
        attributes: BTreeMap::new(),
        ignored_roles,
        ignored_attributes: Vec::new(),
        former_name: document.contract == SecurityContextContract::FormerSecurityContext,
    };
    let mut names: Vec<&String> = document.attributes.keys().collect();
    names.sort_by(|left, right| compare_names(left, right));
    for name in names {
        match declared_attributes.get(name.as_str()) {
            Some(declared) => {
                let typed = check_value(&document.attributes[name], *declared)
                    .map_err(|mismatch| mismatch_error(name, &mismatch))?;
                admitted.attributes.insert(name.clone(), typed);
            }
            None => admitted.ignored_attributes.push(name.clone()),
        }
    }
    Ok(admitted)
}

/// The roles `is_declared` accepts and the others, each in CON-5 order.
fn split_roles(roles: &[String], is_declared: impl Fn(&str) -> bool) -> (Vec<Role>, Vec<Role>) {
    let mut sorted: Vec<&String> = roles.iter().collect();
    sorted.sort_by(|left, right| compare_names(left, right));
    sorted.dedup();
    let (declared, ignored): (Vec<&String>, Vec<&String>) =
        sorted.into_iter().partition(|role| is_declared(role));
    let to_roles = |names: Vec<&String>| names.into_iter().map(|name| Role::new(name)).collect();
    (to_roles(declared), to_roles(ignored))
}

/// The `ESEC-01` error of attribute `name`, at `/attributes/<name>` or at its item.
fn mismatch_error(name: &str, mismatch: &Mismatch) -> AdmissionError {
    let attribute = format!("/attributes/{}", escape_token(name));
    let (pointer, item) = match mismatch.index {
        Some(index) => (format!("{attribute}/{index}"), format!(", item {index}")),
        None => (attribute, String::new()),
    };
    AdmissionError {
        code: AdmissionCode::Esec01,
        document: InputDocument::SecurityContext,
        pointer,
        line_column: None,
        message: format!("attribute `{name}`{item}: {}", mismatch.message),
    }
}

/// One RFC 6901 reference token: `~` becomes `~0` and `/` becomes `~1`.
fn escape_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}
