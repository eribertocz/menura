//! The model elements a dependency document names, ordered as DEP-2 and DEP-3.2 order them.

use menura_model::internal_model::{FieldPath, NormalizedName};

/// A metric by its normalized name: a model metric has no dataset part, a dataset-scoped metric
/// of the development format has its dataset first (QRY-3.2). Ordered by the first name part,
/// a model metric before the dataset metrics with the same first part (DEP-3.2).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MetricName {
    /// The model metric's name, or the dataset of a dataset metric.
    pub(super) first: NormalizedName,
    /// The dataset metric's own name; `None` for a model metric.
    pub(super) second: Option<NormalizedName>,
}

impl MetricName {
    /// The model metric `name`.
    pub(super) fn of_model(name: &NormalizedName) -> Self {
        Self {
            first: name.clone(),
            second: None,
        }
    }

    /// The metric `name` of `dataset`.
    pub(super) fn of_dataset(dataset: &NormalizedName, name: &NormalizedName) -> Self {
        Self {
            first: dataset.clone(),
            second: Some(name.clone()),
        }
    }

    /// `<name>` or `<dataset>.<metric>`.
    fn written(&self) -> String {
        match &self.second {
            Some(second) => format!("{}.{}", self.first.as_str(), second.as_str()),
            None => self.first.as_str().to_owned(),
        }
    }
}

/// A model element that `model_inputs` may hold. The variant order is the order of kinds in
/// `model_inputs`: `dataset`, then `field`, `metric` and `relationship` (DEP-2, PLN-4.4).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ModelInput {
    /// A dataset.
    Dataset(NormalizedName),
    /// A field of a dataset.
    Field(FieldPath),
    /// A model or dataset metric.
    Metric(MetricName),
    /// A relationship.
    Relationship(NormalizedName),
}

impl ModelInput {
    /// The element as `object` and `model_inputs` write it, with normalized names.
    pub(super) fn written(&self) -> String {
        match self {
            Self::Dataset(name) => format!("dataset:{}", name.as_str()),
            Self::Field(path) => format!("field:{}.{}", path.dataset.as_str(), path.field.as_str()),
            Self::Metric(name) => format!("metric:{}", name.written()),
            Self::Relationship(name) => format!("relationship:{}", name.as_str()),
        }
    }
}

/// A row policy: its dataset, then its normalized name, ordered as PLN-4.7 orders them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct PolicyName {
    /// The dataset that declares it.
    pub(super) dataset: NormalizedName,
    /// Its normalized name (E0D-1.4).
    pub(super) policy: NormalizedName,
}

impl PolicyName {
    /// `<dataset>.<policy>`, as `security_policies` writes it (PLN-4.7).
    pub(super) fn written(&self) -> String {
        format!("{}.{}", self.dataset.as_str(), self.policy.as_str())
    }
}
