//! The classification of context metrics (CTX-5.1.6, CTX-5.1.7): a context metric is not a
//! base metric, so E0G-2 does not apply. It is unusable when its base is, with the base's
//! query error; otherwise its home datasets are those of its base, it is recomposable only if
//! its base is (CTX-3.6), and its recorded additivity is non-additive.

use super::derived::{Classifier, Reading, Status};
use super::output::{Additivity, Classification, ClassifiedMetric, MetricKind};
use crate::references::context::ContextMetric;

impl Classifier<'_, '_> {
    /// Classifies the context metric `metric` once its base is.
    pub(super) fn classify_context(
        &mut self,
        metric: &ContextMetric,
        classification: &mut Classification,
    ) -> Status {
        let base = match self.status(&metric.base, classification) {
            Status::Unusable(error) => {
                let reading = Reading {
                    pointer: &metric.pointer,
                    description: format!("the context metric `{}`", metric.name),
                    location: metric.diagnostic_location,
                };
                self.record_unusable(&reading, &metric.base, error, classification);
                return Status::Unusable(error);
            }
            Status::Unknown => return Status::Unknown,
            Status::Classified(index) => index,
        };
        let Some(base) = classification.metrics.get(base) else {
            return Status::Unknown;
        };
        let Some(typed) = self
            .input
            .derived_typing
            .elements
            .iter()
            .find(|element| element.pointer == metric.pointer)
        else {
            return Status::Unknown;
        };
        let Some(result_type) = typed.typed.logical().cloned() else {
            return Status::Unknown;
        };
        let classified = ClassifiedMetric {
            kind: MetricKind::Metric,
            pointer: metric.pointer.clone(),
            components: Vec::new(),
            inputs: vec![metric.base.clone()],
            home_datasets: base.home_datasets.clone(),
            additivity: Additivity::NonAdditive,
            declaration: None,
            recomposable: base.recomposable,
            result_type,
            nullable: typed.typed.nullable,
        };
        classification.metrics.push(classified);
        Status::Classified(classification.metrics.len() - 1)
    }
}
