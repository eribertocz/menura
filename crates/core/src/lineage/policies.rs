//! The row policies applied to the rows of the datasets a column reads (PLN-4.7).

use std::collections::BTreeSet;

use menura_contracts::normalize_unquoted_identifier;
use menura_model::internal_model::{InternalModel, NormalizedName, RoleName, RowPolicy};
use menura_security::AdmittedSecurityContext;

use super::names::written_name;

/// Every row policy of `datasets` that is applicable to the user of `context` (E4-6.9): it
/// remains after exemption and either has no `roles` or lists one of the user's roles. Each is
/// written `<dataset>.<policy>` with normalized names (E0H-1.1), ordered in CON-5 order of the normalized dataset name, then of
/// the normalized policy name (PLN-4.7).
pub(super) fn applied_policies(
    datasets: &BTreeSet<NormalizedName>,
    model: &InternalModel,
    context: &AdmittedSecurityContext,
) -> Vec<String> {
    let mut applied = BTreeSet::new();
    for dataset in datasets.iter().filter_map(|name| model.dataset(name)) {
        for policy in &dataset.row_policies {
            if is_applicable(policy, context) {
                let policy_name = normalized_policy_name(&policy.name);
                let written = format!("{}.{policy_name}", written_name(&dataset.name));
                applied.insert((dataset.name.clone(), policy_name, written));
            }
        }
    }
    applied.into_iter().map(|(_, _, written)| written).collect()
}

/// Whether `policy` is applicable to the user of `context` (E4-6.9).
fn is_applicable(policy: &RowPolicy, context: &AdmittedSecurityContext) -> bool {
    let holds_one = |roles: &[RoleName]| {
        roles.iter().any(|role| {
            context
                .roles()
                .iter()
                .any(|held| held.as_str() == role.as_str())
        })
    };
    let exempt = policy.exempt_roles.as_deref().is_some_and(holds_one);
    !exempt && policy.roles.as_deref().is_none_or(holds_one)
}

/// The normalized name of the declared policy name `name` (E0D-1.4, E0D-1.1): a delimited
/// name without its quotes, `""` standing for `"`, and any other name in upper case.
pub(crate) fn normalized_policy_name(name: &str) -> String {
    match name
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(inner) if !inner.is_empty() => inner.replace("\"\"", "\""),
        _ => normalize_unquoted_identifier(name),
    }
}
