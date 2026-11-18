//! The classification of derived metrics (E0G-5.2, E0G-5.3, DER-5.1.5, DER-5.1.6) and of
//! context metrics (CTX-5.1.6, CTX-5.1.7, in [`super::context`]): each is classified from the
//! metrics it reads, which are classified first, since E0.f rejected every cycle.

use std::collections::{BTreeSet, HashMap};

use super::diagnostic::ClassificationDiagnostic;
use super::input::ClassificationInput;
use super::output::{
    Additivity, Classification, ClassifiedMetric, MetricKind, QueryError, UnusableMetric,
};
use crate::pointer::{JsonPointer, resolve};
use crate::read::{Location, Node};
use crate::references::derived::DerivedMetric;
use crate::structure::Structure;

/// What classification found for a metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Status {
    /// Classified, at this index of [`Classification::metrics`].
    Classified(usize),
    /// Unusable with this query error.
    Unusable(QueryError),
    /// Neither: a metric left to another extension, or a derived metric that E0.f could not
    /// type for a reason reported elsewhere.
    Unknown,
}

/// Classifies every derived and context metric of `input`, adding each to `classification`,
/// which already holds the base metrics.
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
    for metric in &input.context.metrics {
        classifier.status(&metric.pointer, classification);
    }
}

/// The classification of derived metrics in progress, with what was found for each metric.
pub(super) struct Classifier<'run, 'document> {
    document: &'document Node,
    structure: &'run Structure<'document>,
    pub(super) input: &'run ClassificationInput<'run>,
    statuses: HashMap<JsonPointer, Status>,
}

impl Classifier<'_, '_> {
    /// What classification finds for the metric at `pointer`, classifying it first when it is
    /// a derived or context metric not yet classified.
    pub(super) fn status(
        &mut self,
        pointer: &JsonPointer,
        classification: &mut Classification,
    ) -> Status {
        if let Some(status) = self.statuses.get(pointer) {
            return *status;
        }
        let status = if let Some(error) = self.input.earlier_error(self.structure, pointer) {
            Status::Unusable(error)
        } else if let Some(metric) = self.input.derived.metric(pointer) {
            self.classify(metric, classification)
        } else if let Some(metric) = self.input.context.metric(pointer) {
            self.classify_context(metric, classification)
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
        if let Some(status) = self.unusable_input(metric, &inputs, classification) {
            return status;
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

    /// The status of `metric` when one of its `inputs` is unusable: unusable with the query
    /// error of the first of them in the order written, recorded in `classification`
    /// (DER-5.1.5).
    fn unusable_input(
        &self,
        metric: &DerivedMetric,
        inputs: &[(&JsonPointer, Status)],
        classification: &mut Classification,
    ) -> Option<Status> {
        let (input, error) = inputs.iter().find_map(|(input, status)| match status {
            Status::Unusable(error) => Some((*input, *error)),
            _ => None,
        })?;
        let reading = Reading {
            pointer: &metric.pointer,
            description: format!("the derived metric `{}`", metric.name),
            location: metric.diagnostic_location,
        };
        self.record_unusable(&reading, input, error, classification);
        Some(Status::Unusable(error))
    }

    /// Records that the metric of `reading` is unusable because the metric it reads at `input`
    /// is, with that metric's query error and a `W0G-01` at the metric (DER-5.1.5, CTX-5.1.6).
    pub(super) fn record_unusable(
        &self,
        reading: &Reading<'_>,
        input: &JsonPointer,
        error: QueryError,
        classification: &mut Classification,
    ) {
        let node = resolve(self.document, reading.pointer);
        classification.unusable.push(UnusableMetric {
            kind: MetricKind::Metric,
            pointer: reading.pointer.clone(),
            query_error: error,
        });
        classification.warnings.push(ClassificationDiagnostic {
            code: error,
            pointer: reading.pointer.clone(),
            location: node.map_or(reading.location, |node| node.location),
            message: format!(
                "{} reads the metric at `{input}`, which is unusable ({})",
                reading.description,
                error.as_str()
            ),
        });
    }
}

/// A metric that reads another, for the diagnostic that makes it unusable.
pub(super) struct Reading<'metric> {
    /// The JSON Pointer of the metric.
    pub(super) pointer: &'metric JsonPointer,
    /// The metric in words, such as ``the derived metric `rate` ``.
    pub(super) description: String,
    /// Where the metric's diagnostics are located when its node is not found.
    pub(super) location: Location,
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
