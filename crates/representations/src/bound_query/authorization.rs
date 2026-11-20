//! The authorization decisions of E2 (E2-4.7) and the authorized subgraph (E2-4.8).

use menura_contracts::types::audit_record::Access;
use menura_model::internal_model::{AccessMode, FieldPath, NormalizedName, RoleName};

use super::elements::MetricPath;

/// An object the query uses, as authorized.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AuthorizedObject {
    /// A dataset (E2-4.1).
    Dataset(NormalizedName),
    /// A field (E2-4.2).
    Field(FieldPath),
    /// A metric (E2-4.3).
    Metric(MetricPath),
}

/// The model rule that took an authorization decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationRule {
    /// The dataset's `access` (SCY-3), with its mode.
    DatasetAccess(AccessMode),
    /// The dataset has no `access`, so it is inaccessible (SCY-3.0).
    NoDatasetAccess,
    /// A field restriction of the dataset (SCY-3.1), with the roles it admits.
    FieldRestriction(Vec<RoleName>),
    /// No field restriction names the field.
    NoFieldRestriction,
    /// The decisions on the datasets and fields the metric's components read (E2-4.3).
    MetricComponents,
}

/// One authorization decision, recorded for the audit record (E2-4.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizationDecision {
    /// The object decided on.
    pub object: AuthorizedObject,
    /// Granted or denied.
    pub access: Access,
    /// The rule that decided.
    pub rule: AuthorizationRule,
}

/// The datasets the user may use and the relationships between two of them (E2-4.8).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthorizedSubgraph {
    datasets: Vec<NormalizedName>,
    relationships: Vec<NormalizedName>,
}

impl AuthorizedSubgraph {
    /// The subgraph of `datasets` and `relationships`, each ordered by name without repeats.
    #[must_use]
    pub fn new(datasets: Vec<NormalizedName>, relationships: Vec<NormalizedName>) -> Self {
        Self {
            datasets: ordered_without_repeats(datasets),
            relationships: ordered_without_repeats(relationships),
        }
    }

    /// The authorized datasets, ordered by name.
    #[must_use]
    pub fn datasets(&self) -> &[NormalizedName] {
        &self.datasets
    }

    /// The authorized relationships, ordered by name.
    #[must_use]
    pub fn relationships(&self) -> &[NormalizedName] {
        &self.relationships
    }

    /// Whether the dataset `name` is authorized.
    #[must_use]
    pub fn contains_dataset(&self, name: &NormalizedName) -> bool {
        self.datasets.binary_search(name).is_ok()
    }

    /// Whether the relationship `name` is authorized.
    #[must_use]
    pub fn contains_relationship(&self, name: &NormalizedName) -> bool {
        self.relationships.binary_search(name).is_ok()
    }
}

fn ordered_without_repeats(mut names: Vec<NormalizedName>) -> Vec<NormalizedName> {
    names.sort();
    names.dedup();
    names
}
