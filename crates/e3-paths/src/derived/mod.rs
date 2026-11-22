//! The expansion of a metric into its base components: a derived metric is replaced by the
//! components of its inputs, down to metrics that are not derived (DER-2.1, DER-2.2, DER-2.4).

mod formula;

use menura_contracts::normalize_unquoted_identifier;
use menura_model::expression::{Identifier, Reference};
use menura_model::internal_model::{Component, InternalModel, Metric, NormalizedName};
use menura_representations::bound_query::MetricPath;

use crate::derived::formula::metric_inputs;

/// A metric as a query or an `AGG(…)` term names it: of the semantic model when `dataset` is
/// `None`, of `dataset` otherwise (DER-5.1.2).
#[derive(Debug, Clone, PartialEq, Eq)]
struct MetricKey {
    dataset: Option<NormalizedName>,
    metric: NormalizedName,
}

impl MetricKey {
    fn of_path(path: &MetricPath) -> Self {
        match path {
            MetricPath::SemanticModel(metric) => Self {
                dataset: None,
                metric: metric.clone(),
            },
            MetricPath::Dataset { dataset, metric } => Self {
                dataset: Some(dataset.clone()),
                metric: metric.clone(),
            },
        }
    }

    fn of_term(reference: &Reference) -> Self {
        Self {
            dataset: reference.qualifier.as_ref().map(normalized),
            metric: normalized(&reference.name),
        }
    }
}

/// Why a metric cannot be expanded; both are ruled out before E3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpansionFailure {
    /// The metric, or one of its inputs, is not in the model or was not classified.
    UnknownMetric,
    /// The inputs of a derived metric lead back to it (`E0C-34`, DER-5.1.4).
    Cycle,
}

/// A base component with the metric that is not derived whose expression it belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BaseComponent<'model> {
    /// The base metric.
    pub(crate) metric: &'model Metric,
    /// The component.
    pub(crate) component: &'model Component,
}

/// The base components of the metric `path` names, in expansion order: for a metric that is
/// not derived, its own components in written order (E0G-1.1); for a derived metric, the
/// components of each input in the order its `AGG(…)` terms are written, each input expanded
/// once, recursively (DER-2.4).
///
/// # Errors
///
/// An [`ExpansionFailure`] for a metric E2 and E0 never let through.
pub(crate) fn base_components<'model>(
    path: &MetricPath,
    model: &'model InternalModel,
) -> Result<Vec<BaseComponent<'model>>, ExpansionFailure> {
    let mut expansion = Expansion {
        model,
        chain: Vec::new(),
        expanded: Vec::new(),
        components: Vec::new(),
    };
    expansion.expand(MetricKey::of_path(path))?;
    Ok(expansion.components)
}

/// The state of one expansion.
struct Expansion<'model> {
    model: &'model InternalModel,
    /// The derived metrics being expanded, outermost first.
    chain: Vec<MetricKey>,
    /// The inputs already expanded.
    expanded: Vec<MetricKey>,
    /// The base components found so far.
    components: Vec<BaseComponent<'model>>,
}

impl Expansion<'_> {
    fn expand(&mut self, key: MetricKey) -> Result<(), ExpansionFailure> {
        let metric = find(&key, self.model).ok_or(ExpansionFailure::UnknownMetric)?;
        let classification = metric
            .classification
            .as_ref()
            .ok_or(ExpansionFailure::UnknownMetric)?;
        let inputs = metric
            .expression
            .as_ref()
            .map(metric_inputs)
            .unwrap_or_default();
        if inputs.is_empty() {
            self.components.extend(
                classification
                    .components
                    .iter()
                    .map(|component| BaseComponent { metric, component }),
            );
            return Ok(());
        }
        self.chain.push(key);
        for input in inputs {
            let input = MetricKey::of_term(input);
            if self.chain.contains(&input) {
                return Err(ExpansionFailure::Cycle);
            }
            if self.expanded.contains(&input) {
                continue;
            }
            self.expand(input.clone())?;
            self.expanded.push(input);
        }
        self.chain.pop();
        Ok(())
    }
}

/// The metric `key` names in `model`.
fn find<'model>(key: &MetricKey, model: &'model InternalModel) -> Option<&'model Metric> {
    match &key.dataset {
        None => model.metric(&key.metric),
        Some(dataset) => model
            .dataset(dataset)?
            .metrics
            .iter()
            .find(|candidate| candidate.name == key.metric),
    }
}

/// The name an identifier refers to (E0D-1.1): a regular one in upper case, a delimited one
/// exactly.
fn normalized(identifier: &Identifier) -> NormalizedName {
    if identifier.is_delimited() {
        NormalizedName::new(identifier.text())
    } else {
        NormalizedName::new(&normalize_unquoted_identifier(identifier.text()))
    }
}
