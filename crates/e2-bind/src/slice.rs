//! Row 12c of E2-5: the 0.6.0 slice (VS-1d), which is the 0.5.0 slice (VS-1c) unchanged. The
//! first element in query order that needs a context metric (0.8.0) is `E_NOT_YET_SUPPORTED`,
//! located at that element (E2-5.4, VS-2). From 0.5.0, context metrics are the only capability
//! outside the slice, and none belongs to the whole query (E2-5.4a).

use menura_model::internal_model::{InternalModel, Metric};

use crate::authorized_query::ResolvedName;
use crate::error::{BindCode, BindDiagnostic};
use crate::model_elements::metric;
use crate::named::{NamedElement, NamedTarget};
use crate::read_metrics::metric_closure;

/// The capability outside the 0.6.0 slice, as a message names it.
const CONTEXT_METRIC: &str = "context metrics";

/// The version that adds it (table 7.1).
const CONTEXT_METRIC_VERSION: &str = "0.8.0";

/// Row 12c over `named`, in query order: the first element that needs a context metric is
/// reported at that element.
pub(crate) fn check_slice(
    model: &InternalModel,
    named: &[NamedElement],
) -> Result<(), BindDiagnostic> {
    match named
        .iter()
        .find(|element| needs_context_metric(model, &element.target))
    {
        Some(element) => Err(not_yet_supported(element)),
        None => Ok(()),
    }
}

/// Whether `target` is a metric that is, or reads, a context metric (E2-5.4); fields, datasets
/// and relationships need none.
fn needs_context_metric(model: &InternalModel, target: &NamedTarget) -> bool {
    let NamedTarget::Member(ResolvedName::Metric(path)) = target else {
        return false;
    };
    metric(model, path).is_some_and(|root| {
        metric_closure(model, root)
            .into_iter()
            .any(is_context_metric)
    })
}

fn is_context_metric(metric: &Metric) -> bool {
    metric.context.is_some()
}

fn not_yet_supported(element: &NamedElement) -> BindDiagnostic {
    BindDiagnostic::new(
        BindCode::NotYetSupported,
        format!(
            "this query needs {CONTEXT_METRIC}, which is not supported before \
             {CONTEXT_METRIC_VERSION}"
        ),
        &element.origin,
    )
}
