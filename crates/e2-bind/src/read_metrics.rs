//! The metrics a metric reads, recursively: through `AGG(…)` in its expression, which covers the
//! inputs of a derived metric (DER-5.1.2) and the base of a context metric (CTX-2.1), and
//! through the `base` of its context extension. Row 12c gives a metric the capabilities of
//! every metric it reads (E2-5.4).

use std::collections::BTreeSet;

use menura_model::expression::Expression;
use menura_model::internal_model::{InternalModel, Metric};
use menura_representations::bound_query::MetricPath;

use crate::model_elements::metric;
use crate::names::identifier_name;
use crate::walk::nodes;

/// `root` and every metric it reads, recursively, each once: `root` first, then in the order
/// they are reached. A name the model does not hold is skipped; E0 marks such a metric
/// unusable, which row 12a reports first.
pub(crate) fn metric_closure<'model>(
    model: &'model InternalModel,
    root: &'model Metric,
) -> Vec<&'model Metric> {
    let mut visited: BTreeSet<MetricPath> = BTreeSet::new();
    let mut closure = vec![root];
    let mut index = 0;
    while let Some(current) = closure.get(index) {
        for path in direct_reads(current) {
            if visited.insert(path.clone())
                && let Some(found) = metric(model, &path)
                && !std::ptr::eq(found, root)
            {
                closure.push(found);
            }
        }
        index += 1;
    }
    closure
}

/// The metrics `metric` reads directly, in written order, then its context base.
fn direct_reads(metric: &Metric) -> Vec<MetricPath> {
    let mut reads: Vec<MetricPath> = metric
        .expression
        .as_ref()
        .map(|expression| {
            nodes(expression)
                .into_iter()
                .filter_map(aggregated_metric)
                .collect()
        })
        .unwrap_or_default();
    if let Some(context) = &metric.context {
        reads.push(MetricPath::SemanticModel(context.base.clone()));
    }
    reads
}

/// The metric `AGG(…)` names: `AGG(dataset.metric)` a dataset metric, `AGG(metric)` a metric of
/// the semantic model (DER-5.1.2).
fn aggregated_metric(node: &Expression) -> Option<MetricPath> {
    let Expression::MetricAggregate(reference) = node else {
        return None;
    };
    let name = identifier_name(&reference.name);
    Some(match &reference.qualifier {
        Some(dataset) => MetricPath::Dataset {
            dataset: identifier_name(dataset),
            metric: name,
        },
        None => MetricPath::SemanticModel(name),
    })
}
