//! The `authorization` entries of the audit record (PLN-6.5), written from the decisions E2
//! recorded (E2-4.7).

use menura_contracts::types::audit_record::AuthorizationDecision as AuditEntry;
use menura_model::internal_model::{AccessMode, NormalizedName, RoleName};
use menura_representations::bound_query::{
    AuthorizationDecision, AuthorizationRule, AuthorizedObject, MetricPath,
};

/// The audit entries of `decisions`, in the order E2 took them (PLN-6.5).
pub(super) fn authorization_entries(decisions: &[AuthorizationDecision]) -> Vec<AuditEntry> {
    decisions
        .iter()
        .map(|decision| AuditEntry {
            object: object(&decision.object),
            decision: decision.access,
            rule: rule(&decision.rule),
        })
        .collect()
}

/// `dataset:<name>`, `field:<dataset>.<field>` or `metric:<name>`, with normalized names
/// (E0H-1.1).
fn object(object: &AuthorizedObject) -> String {
    match object {
        AuthorizedObject::Dataset(name) => format!("dataset:{}", name.as_str()),
        AuthorizedObject::Field(path) => {
            format!("field:{}.{}", path.dataset.as_str(), path.field.as_str())
        }
        AuthorizedObject::Metric(path) => format!("metric:{}", metric_name(path).as_str()),
    }
}

/// The name of the metric `path`, without the dataset that declares it.
fn metric_name(path: &MetricPath) -> &NormalizedName {
    match path {
        MetricPath::SemanticModel(name) | MetricPath::Dataset { metric: name, .. } => name,
    }
}

/// The written rule of PLN-6.5.
fn rule(rule: &AuthorizationRule) -> String {
    match rule {
        AuthorizationRule::DatasetAccess(mode) => format!("access:{}", mode_name(mode)),
        AuthorizationRule::NoDatasetAccess => "no_access".to_owned(),
        AuthorizationRule::FieldRestriction(roles) => {
            format!("field_access:{}", sorted_roles(roles))
        }
        AuthorizationRule::NoFieldRestriction => "no_field_restriction".to_owned(),
        AuthorizationRule::MetricComponents => "metric_components".to_owned(),
    }
}

/// The `access` mode as the model writes it.
fn mode_name(mode: &AccessMode) -> &'static str {
    match mode {
        AccessMode::Authenticated => "authenticated",
        AccessMode::Roles(_) => "roles",
        AccessMode::Internal => "internal",
    }
}

/// `roles` sorted by code points, without duplicates, joined by `,`.
fn sorted_roles(roles: &[RoleName]) -> String {
    let mut names: Vec<&str> = roles.iter().map(RoleName::as_str).collect();
    names.sort_unstable();
    names.dedup();
    names.join(",")
}

#[cfg(test)]
mod tests {
    use menura_contracts::types::audit_record::Access;
    use menura_model::internal_model::{FieldPath, NormalizedName, RoleName};

    use super::*;

    fn name(text: &str) -> NormalizedName {
        NormalizedName::new(text)
    }

    fn decision(object: AuthorizedObject, rule: AuthorizationRule) -> AuthorizationDecision {
        AuthorizationDecision {
            object,
            access: Access::Granted,
            rule,
        }
    }

    #[test]
    fn pln_6_5_objects_are_written_with_normalized_names() {
        let entries = authorization_entries(&[
            decision(
                AuthorizedObject::Dataset(name("ORDERS")),
                AuthorizationRule::NoDatasetAccess,
            ),
            decision(
                AuthorizedObject::Field(FieldPath {
                    dataset: name("ORDERS"),
                    field: name("REGION"),
                }),
                AuthorizationRule::NoFieldRestriction,
            ),
            decision(
                AuthorizedObject::Metric(MetricPath::Dataset {
                    dataset: name("ORDERS"),
                    metric: name("TOTAL"),
                }),
                AuthorizationRule::MetricComponents,
            ),
        ]);
        let objects: Vec<&str> = entries.iter().map(|entry| entry.object.as_str()).collect();
        assert_eq!(
            objects,
            ["dataset:ORDERS", "field:ORDERS.REGION", "metric:TOTAL"]
        );
        let rules: Vec<&str> = entries.iter().map(|entry| entry.rule.as_str()).collect();
        assert_eq!(
            rules,
            ["no_access", "no_field_restriction", "metric_components"]
        );
    }

    #[test]
    fn pln_6_5_access_rules_name_their_mode() {
        for (mode, written) in [
            (AccessMode::Authenticated, "access:authenticated"),
            (
                AccessMode::Roles(vec![RoleName::new("admin")]),
                "access:roles",
            ),
            (AccessMode::Internal, "access:internal"),
        ] {
            assert_eq!(rule(&AuthorizationRule::DatasetAccess(mode)), written);
        }
    }

    #[test]
    fn pln_6_5_field_access_roles_are_sorted_without_duplicates() {
        let roles = ["b", "B", "a", "b"].map(RoleName::new).to_vec();
        assert_eq!(
            rule(&AuthorizationRule::FieldRestriction(roles)),
            "field_access:B,a,b"
        );
    }
}
