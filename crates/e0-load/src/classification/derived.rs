//! The classification of derived metrics (E0G-5.2, E0G-5.3, DER-5.1.5, DER-5.1.6): each is
//! classified from its inputs, which are classified first, since E0.f rejected every cycle.

use std::collections::{BTreeSet, HashMap};

use super::diagnostic::ClassificationDiagnostic;
use super::input::ClassificationInput;
use super::output::{
    Additivity, Classification, ClassifiedMetric, MetricKind, QueryError, UnusableMetric,
};
use crate::pointer::{JsonPointer, resolve};
use crate::read::Node;
use crate::references::derived::DerivedMetric;
use crate::structure::Structure;

/// What classification found for a metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Status {
    /// Classified, at this index of [`Classification::metrics`].
    Classified(usize),
    /// Unusable with this query error.
    Unusable(QueryError),
    /// Neither: a metric left to another extension, or a derived metric that E0.f could not
    /// type for a reason reported elsewhere.
    Unknown,
}

/// Classifies every derived metric of `input`, adding each to `classification`, which already
/// holds the base metrics.
pub(super) fn classify_derived(
    document: &Node,
    structure: &Structure<'_>,
    input: &ClassificationInput<'_>,
    classification: &mut Classification,
) {
    let mut classifier = Classifier {
        document,
        structure,
        input,
        statuses: HashMap::new(),
    };
    for metric in &input.derived.metrics {
        classifier.status(&metric.pointer, classification);
    }
}

/// The classification of derived metrics in progress, with what was found for each metric.
struct Classifier<'run, 'document> {
    document: &'document Node,
    structure: &'run Structure<'document>,
    input: &'run ClassificationInput<'run>,
    statuses: HashMap<JsonPointer, Status>,
}

impl Classifier<'_, '_> {
    /// What classification finds for the metric at `pointer`, classifying it first when it is
    /// a derived metric not yet classified.
    fn status(&mut self, pointer: &JsonPointer, classification: &mut Classification) -> Status {
        if let Some(status) = self.statuses.get(pointer) {
            return *status;
        }
        let status = if let Some(error) = self.input.earlier_error(self.structure, pointer) {
            Status::Unusable(error)
        } else if let Some(metric) = self.input.derived.metric(pointer) {
            self.classify(metric, classification)
        } else {
            base_status(pointer, classification)
        };
        self.statuses.insert(pointer.clone(), status);
        status
    }

    /// Classifies the derived metric `metric` once its inputs are: it is unusable when one of
    /// them is (DER-5.1.5), and otherwise classified from them (DER-5.1.6).
    fn classify(&mut self, metric: &DerivedMetric, classification: &mut Classification) -> Status {
        let mut inputs: Vec<(&JsonPointer, Status)> = Vec::new();
        for input in &metric.inputs {
            if inputs.iter().all(|(seen, _)| **seen != input.metric) {
                let status = self.status(&input.metric, classification);
                inputs.push((&input.metric, status));
            }
        }
        if let Some((input, error)) = inputs.iter().find_map(|(input, status)| match status {
            Status::Unusable(error) => Some((*input, *error)),
            _ => None,
        }) {
            self.record_unusable(metric, input, error, classification);
            return Status::Unusable(error);
        }
        let classified: Vec<&ClassifiedMetric> = inputs
            .iter()
            .filter_map(|(_, status)| match status {
                Status::Classified(index) => classification.metrics.get(*index),
                _ => None,
            })
            .collect();
        let typed = self
            .input
            .derived_typing
            .elements
            .iter()
            .find(|element| element.pointer == metric.pointer);
        let (Some(typed), true) = (typed, classified.len() == inputs.len()) else {
            return Status::Unknown;
        };
        let Some(result_type) = typed.typed.logical().cloned() else {
            return Status::Unknown;
        };
        let derived = ClassifiedMetric {
            kind: MetricKind::Metric,
            pointer: metric.pointer.clone(),
            components: Vec::new(),
            inputs: inputs.iter().map(|(input, _)| (*input).clone()).collect(),
            home_datasets: classified
                .iter()
                .flat_map(|input| input.home_datasets.iter().cloned())
                .collect::<BTreeSet<_>>(),
            additivity: Additivity::NonAdditive,
            declaration: None,
            recomposable: classified.iter().all(|input| input.recomposable),
            result_type,
            nullable: typed.typed.nullable,
        };
        classification.metrics.push(derived);
        Status::Classified(classification.metrics.len() - 1)
    }

    /// Records that `metric` is unusable because its input at `input` is, with that input's
    /// query error and a `W0G-01` at the metric (DER-5.1.5).
    fn record_unusable(
        &self,
        metric: &DerivedMetric,
        input: &JsonPointer,
        error: QueryError,
        classification: &mut Classification,
    ) {
        let node = resolve(self.document, &metric.pointer);
        classification.unusable.push(UnusableMetric {
            kind: MetricKind::Metric,
            pointer: metric.pointer.clone(),
            query_error: error,
        });
        classification.warnings.push(ClassificationDiagnostic {
            code: error,
            pointer: metric.pointer.clone(),
            location: node.map_or(metric.diagnostic_location, |node| node.location),
            message: format!(
                "the derived metric `{}` reads the metric at `{input}`, which is unusable ({})",
                metric.name,
                error.as_str()
            ),
        });
    }
}

/// What classification found for the base metric at `pointer`.
fn base_status(pointer: &JsonPointer, classification: &Classification) -> Status {
    if let Some(index) = classification
        .metrics
        .iter()
        .position(|metric| metric.pointer == *pointer)
    {
        return Status::Classified(index);
    }
    classification
        .unusable
        .iter()
        .find(|metric| metric.pointer == *pointer)
        .map_or(Status::Unknown, |metric| {
            Status::Unusable(metric.query_error)
        })
}
