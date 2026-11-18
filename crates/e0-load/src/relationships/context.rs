//! `E0C-61`, in E0.e: the `time_dimension` of a context metric exists, and its dataset is each
//! home dataset of the base or is reached from each by a path (CTX-5.1.4, E0E-2.3, E0E-2.4).
//!
//! The home datasets of the base are those E0.g gives it: the datasets its aggregate arguments
//! read (E0G-1.1), the own dataset of a dataset metric (E0G-1.4), the union of those of the
//! inputs of a derived metric (E0G-5.2), and those of the base of a context metric
//! (CTX-5.1.7). A base that depends on a metric made unusable before E0.e is not checked: the
//! context metric is unusable (CTX-5.1.6).

use std::collections::BTreeSet;

use super::home_datasets::home_datasets;
use super::output::Relationships;
use super::reach::Model;
use crate::extensions::additivity::FieldReference;
use crate::extensions::{ExtensionCode, ExtensionDiagnostic};
use crate::names::Name;
use crate::outline::outline;
use crate::pointer::JsonPointer;
use crate::references::References;
use crate::references::context::{ContextMetric, ContextMetrics};
use crate::references::derived::DerivedMetrics;
use crate::structure::Structure;

/// The metrics E0.d resolved, which the home datasets of a base follow.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedMetrics<'earlier> {
    /// The derived metrics and their inputs, and the metrics `E0C-37` makes unusable.
    pub(crate) derived: &'earlier DerivedMetrics,
    /// The context metrics and their bases.
    pub(crate) context: &'earlier ContextMetrics,
}

/// Checks the time dimension of every context metric of `resolved`, in the semantic model that
/// `structure` selected, with the elements `references` made unusable and the graph of
/// `relationships`.
///
/// # Errors
///
/// Returns one `E0C-61` diagnostic per context metric whose time dimension does not exist or
/// is not reachable, at the entry's `data` member (E0C-2.6), in document order. `E0C-61`
/// rejects the model (E0-4).
pub(crate) fn check_context_reachability(
    structure: &Structure<'_>,
    references: &References,
    resolved: ResolvedMetrics<'_>,
    relationships: &Relationships,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let outline = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let model = Model::new(structure, &outline, references, relationships);
    let errors: Vec<ExtensionDiagnostic> = resolved
        .context
        .metrics
        .iter()
        .filter_map(|metric| unreachable_time_dimension(&model, resolved, metric))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The `E0C-61` diagnostic of `metric`, if its time dimension does not exist or is not
/// reachable from each home dataset of its base.
fn unreachable_time_dimension(
    model: &Model<'_, '_>,
    resolved: ResolvedMetrics<'_>,
    metric: &ContextMetric,
) -> Option<ExtensionDiagnostic> {
    let time_dimension = &metric.modifier.time()?.time_dimension;
    let homes = Homes { model, resolved }.of(&metric.base, &mut Vec::new())?;
    let problem = field_problem(model, time_dimension)
        .or_else(|| path_problem(model, &homes, time_dimension))?;
    Some(ExtensionDiagnostic {
        code: ExtensionCode::ContextTimeDimension,
        pointer: metric.diagnostic_pointer.clone(),
        location: metric.diagnostic_location,
        message: format!(
            "the `time_dimension` of the context metric `{}` is not reachable from its base: {problem}",
            metric.name
        ),
    })
}

/// Why `field` is not a field of the model.
fn field_problem(model: &Model<'_, '_>, field: &FieldReference) -> Option<String> {
    let Some(dataset) = model.datasets.get(&field.dataset) else {
        return Some(format!("`{}` names no dataset", field.written));
    };
    dataset
        .field(&field.field)
        .is_none()
        .then(|| format!("`{}` names no field of its dataset", field.written))
}

/// Why the dataset of `field` is not each of `homes` or reached from each.
fn path_problem(
    model: &Model<'_, '_>,
    homes: &BTreeSet<Name>,
    field: &FieldReference,
) -> Option<String> {
    let unreached: Vec<String> = homes
        .iter()
        .filter(|home| {
            **home != field.dataset && !model.reached_from(home).contains(&field.dataset)
        })
        .map(|home| format!("`{}`", home.as_str()))
        .collect();
    (!unreached.is_empty()).then(|| {
        format!(
            "no path leads to the dataset of `{}` from the home dataset {}",
            field.written,
            unreached.join(", ")
        )
    })
}

/// The home datasets of metrics, following derived inputs and context bases.
struct Homes<'run, 'model, 'document> {
    model: &'run Model<'model, 'document>,
    resolved: ResolvedMetrics<'run>,
}

impl Homes<'_, '_, '_> {
    /// The home datasets of the metric at `metric`, or `None` when it, or a metric it reads, is
    /// unusable before E0.e. `visiting` holds the metrics on the current path: a metric that
    /// reads itself adds nothing, since E0.f rejects the cycle (`E0C-34`).
    fn of(&self, metric: &JsonPointer, visiting: &mut Vec<JsonPointer>) -> Option<BTreeSet<Name>> {
        if visiting.contains(metric) {
            return Some(BTreeSet::new());
        }
        if self.model.unusable.contains(metric) || self.resolved.derived.unusable.contains(metric) {
            return None;
        }
        if let Some(dataset) = self.model.dataset_metrics.get(metric) {
            return Some(BTreeSet::from([(*dataset).clone()]));
        }
        let read: Vec<JsonPointer> = if let Some(derived) = self.resolved.derived.metric(metric) {
            derived
                .inputs
                .iter()
                .map(|input| input.metric.clone())
                .collect()
        } else if let Some(context) = self.resolved.context.metric(metric) {
            vec![context.base.clone()]
        } else {
            return self.model.metrics.get(metric).map(home_datasets);
        };
        visiting.push(metric.clone());
        let mut found = BTreeSet::new();
        for input in &read {
            let Some(homes) = self.of(input, visiting) else {
                visiting.pop();
                return None;
            };
            found.extend(homes);
        }
        visiting.pop();
        Some(found)
    }
}
