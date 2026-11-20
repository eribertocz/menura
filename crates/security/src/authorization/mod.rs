//! Authorization of the objects a query uses (E2-4): datasets by their `access` (SCY-3),
//! fields by their restrictions (SCY-3.1, SCY-3.1a), metrics by what they read (E2-4.3,
//! SCY-5.1.8), and the authorized subgraph (E2-4.8). Deny by default (SEC-3.7).

mod reads;
#[cfg(test)]
mod test_model;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use menura_contracts::types::audit_record::Access;
use menura_model::internal_model::{FieldPath, InternalModel, NormalizedName, RoleName};
use menura_representations::bound_query::{
    AuthorizationDecision, AuthorizationRule, AuthorizedObject, AuthorizedSubgraph, MetricPath,
};

use crate::context::AdmittedSecurityContext;
use crate::restrictions::{Restrictions, restrictions};
use crate::role::Role;
use reads::{ModelReads, model_reads};

/// The authorization of one user against one model: it decides on each dataset, field and
/// metric, and computes the authorized subgraph.
///
/// It decides on the model's declarations and the user's declared roles only (SEC-3.2): a
/// role the model does not declare grants nothing.
#[derive(Debug, Clone)]
pub struct Authorization<'model> {
    model: &'model InternalModel,
    roles: BTreeSet<Role>,
    restrictions: Restrictions<FieldPath, MetricPath>,
    metric_datasets: BTreeMap<MetricPath, BTreeSet<NormalizedName>>,
    metric_inputs: BTreeMap<MetricPath, BTreeSet<MetricPath>>,
}

impl<'model> Authorization<'model> {
    /// The authorization of the user of `context` against `model`.
    #[must_use]
    pub fn new(model: &'model InternalModel, context: &AdmittedSecurityContext) -> Self {
        let ModelReads {
            restriction_input,
            metric_datasets,
        } = model_reads(model);
        Self {
            model,
            roles: context.roles().iter().cloned().collect(),
            restrictions: restrictions(&restriction_input),
            metric_datasets,
            metric_inputs: restriction_input.metric_inputs,
        }
    }

    /// The decision on the dataset `name` (E2-4.1): granted when its `access` admits the user
    /// (SCY-3). A dataset without `access` (SCY-3.0), an `internal` one (E2-4.6) and a name
    /// that is not a dataset are denied.
    #[must_use]
    pub fn dataset(&self, name: &NormalizedName) -> AuthorizationDecision {
        let access = self
            .model
            .dataset(name)
            .and_then(|dataset| dataset.access.as_ref());
        let (granted, rule) = match access {
            Some(access) => (
                self.admits_mode(&access.mode),
                AuthorizationRule::DatasetAccess(access.mode.clone()),
            ),
            None => (false, AuthorizationRule::NoDatasetAccess),
        };
        decision(AuthorizedObject::Dataset(name.clone()), granted, rule)
    }

    /// The decision on the field `path` by its dataset's field restrictions (E2-4.2, SCY-3.1,
    /// SCY-3.1a): granted when no restriction reaches it, or when the user holds a role of the
    /// intersection of those that do. Its dataset is decided separately.
    #[must_use]
    pub fn field(&self, path: &FieldPath) -> AuthorizationDecision {
        let object = AuthorizedObject::Field(path.clone());
        match self.restrictions.fields.get(path) {
            Some(roles) => decision(
                object,
                self.holds_any(roles),
                AuthorizationRule::FieldRestriction(role_names(roles)),
            ),
            None => decision(object, true, AuthorizationRule::NoFieldRestriction),
        }
    }

    /// The decision on the metric `path` (E2-4.3): granted when every dataset it and its input
    /// metrics read is authorized, and the user holds a role of its restriction when it has
    /// one (SCY-5.1.8).
    #[must_use]
    pub fn metric(&self, path: &MetricPath) -> AuthorizationDecision {
        let datasets_granted = self
            .datasets_read(path)
            .iter()
            .all(|dataset| self.dataset(dataset).access == Access::Granted);
        let restriction_granted = self
            .restrictions
            .metrics
            .get(path)
            .is_none_or(|roles| self.holds_any(roles));
        decision(
            AuthorizedObject::Metric(path.clone()),
            datasets_granted && restriction_granted,
            AuthorizationRule::MetricComponents,
        )
    }

    /// The authorized subgraph (E2-4.8): the datasets whose `access` admits the user, and the
    /// relationships whose two ends are both such datasets.
    #[must_use]
    pub fn subgraph(&self) -> AuthorizedSubgraph {
        let parts = self.model.parts();
        let datasets: Vec<NormalizedName> = parts
            .datasets
            .iter()
            .map(|dataset| &dataset.name)
            .filter(|name| self.dataset(name).access == Access::Granted)
            .cloned()
            .collect();
        let admitted = |name: &NormalizedName| datasets.binary_search(name).is_ok();
        let relationships = parts
            .relationships
            .iter()
            .filter(|relationship| admitted(&relationship.from) && admitted(&relationship.to))
            .map(|relationship| relationship.name.clone())
            .collect();
        AuthorizedSubgraph::new(datasets, relationships)
    }

    /// Every dataset `path` reads, with those of its input metrics, without repeats.
    fn datasets_read(&self, path: &MetricPath) -> BTreeSet<&NormalizedName> {
        let mut datasets = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut pending = vec![path];
        while let Some(metric) = pending.pop() {
            if !visited.insert(metric) {
                continue;
            }
            datasets.extend(self.metric_datasets.get(metric).into_iter().flatten());
            pending.extend(self.metric_inputs.get(metric).into_iter().flatten());
        }
        datasets
    }

    /// Whether a dataset `mode` admits the user (SCY-3).
    fn admits_mode(&self, mode: &menura_model::internal_model::AccessMode) -> bool {
        use menura_model::internal_model::AccessMode;
        match mode {
            AccessMode::Authenticated => true,
            AccessMode::Roles(roles) => roles
                .iter()
                .any(|role| self.roles.contains(&Role::new(role.as_str()))),
            AccessMode::Internal => false,
        }
    }

    /// Whether the user holds one of `roles`.
    fn holds_any(&self, roles: &BTreeSet<Role>) -> bool {
        !self.roles.is_disjoint(roles)
    }
}

/// The decision on `object`.
fn decision(
    object: AuthorizedObject,
    granted: bool,
    rule: AuthorizationRule,
) -> AuthorizationDecision {
    let access = if granted {
        Access::Granted
    } else {
        Access::Denied
    };
    AuthorizationDecision {
        object,
        access,
        rule,
    }
}

/// `roles` as the model names them.
fn role_names(roles: &BTreeSet<Role>) -> Vec<RoleName> {
    roles
        .iter()
        .map(|role| RoleName::new(role.as_str()))
        .collect()
}
