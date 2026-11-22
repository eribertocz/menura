//! `E0C-64` and `E0C-65`: what a context base reads through derived metrics (CTX-5.1.3).
//!
//! Version 1 allows one level of context, so a base that is a context metric, or reads one
//! through derived metrics, is `E0C-64`. A running total of snapshots has no meaning, so a
//! `period_to_date` base that is semi-additive, or reads a semi-additive metric through
//! derived metrics, is `E0C-65`.

use std::collections::HashSet;

use menura_model::expression::Reference;

use super::Model;
use crate::extensions::additivity::DeclaredAdditivity;
use crate::extensions::context::{ContextDeclaration, Modifier};
use crate::extensions::{ExtensionCode, Extensions};
use crate::names::Name;
use crate::outline::ElementOutline;
use crate::pointer::JsonPointer;
use crate::references::walk::metric_aggregates;

/// The base `base` and every metric it reads through derived metrics, recursively, each once:
/// the base first, then in the order of a depth-first walk of the `AGG(…)` terms. Only a
/// derived metric is walked into; a term that names no metric is reported by E0C-32.
pub(super) fn read_metrics<'model, 'document>(
    model: &Model<'model, 'document>,
    base: &'model ElementOutline<'document>,
) -> Vec<&'model ElementOutline<'document>> {
    let mut seen: HashSet<&JsonPointer> = HashSet::from([&base.pointer]);
    let mut read = vec![base];
    let mut pending = vec![base];
    while let Some(metric) = pending.pop() {
        if !model.derived.contains(&metric.pointer) {
            continue;
        }
        let Some(tree) = model.trees.get(&metric.pointer) else {
            continue;
        };
        for term in metric_aggregates(tree) {
            if let Some(input) = named_metric(model, term)
                && seen.insert(&input.pointer)
            {
                read.push(input);
                pending.push(input);
            }
        }
    }
    read
}

/// The metric `AGG(term)` names: a model metric, or a dataset metric of the development
/// format, compared under E0D-1.1 and E0D-1.3.
fn named_metric<'model, 'document>(
    model: &Model<'model, 'document>,
    term: &Reference,
) -> Option<&'model ElementOutline<'document>> {
    let name = Name::of(&term.name);
    match &term.qualifier {
        None => model.metrics.get(&name).copied(),
        Some(dataset) => model
            .datasets
            .get(&Name::of(dataset))?
            .metrics
            .iter()
            .find(|metric| metric.name.as_ref().is_some_and(|found| found.name == name)),
    }
}

/// The `E0C-64` problem of a base whose metrics `read` ([`read_metrics`]) hold a context
/// metric. A derived base that reads back the metric that declares it is a cycle, reported as
/// `E0C-34` by E0.f (CTX-5.1.5), not a second level.
pub(super) fn nested_context_problem(
    extensions: &Extensions,
    declaration: &ContextDeclaration,
    read: &[&ElementOutline<'_>],
) -> Option<(ExtensionCode, String)> {
    let (base, through) = read.split_first()?;
    let message = if extensions.is_context_metric(&base.pointer) {
        format!(
            "`base` `{}` is itself a context metric; a context metric has one level",
            declaration.base
        )
    } else {
        let nested = through.iter().find(|metric| {
            metric.pointer != declaration.metric && extensions.is_context_metric(&metric.pointer)
        })?;
        format!(
            "`base` `{}` reads the context metric `{}` through derived metrics; a context metric has one level",
            declaration.base,
            written_name(nested)
        )
    };
    Some((ExtensionCode::NestedContext, message))
}

/// The `E0C-65` problem of a `period_to_date` metric whose base reads, among the metrics
/// `read` ([`read_metrics`]), a semi-additive metric.
pub(super) fn running_total_problem(
    extensions: &Extensions,
    declaration: &ContextDeclaration,
    read: &[&ElementOutline<'_>],
) -> Option<(ExtensionCode, String)> {
    if !matches!(declaration.modifier, Modifier::PeriodToDate(_)) {
        return None;
    }
    let snapshot = read
        .iter()
        .find(|metric| is_semi_additive(extensions, &metric.pointer))?;
    Some((
        ExtensionCode::SemiAdditiveRunningTotal,
        format!(
            "`base` `{}` reads the semi-additive metric `{}`; a running total of snapshots has no meaning",
            declaration.base,
            written_name(snapshot)
        ),
    ))
}

/// Whether the metric at `metric` declares `additivity` with `type: semi_additive`.
fn is_semi_additive(extensions: &Extensions, metric: &JsonPointer) -> bool {
    extensions.additivity.iter().any(|declaration| {
        declaration.metric == *metric
            && matches!(declaration.additivity, DeclaredAdditivity::SemiAdditive(_))
    })
}

/// The name of `metric` as written.
fn written_name<'document>(metric: &ElementOutline<'document>) -> &'document str {
    metric.name.as_ref().map_or("", |name| name.written.text)
}
