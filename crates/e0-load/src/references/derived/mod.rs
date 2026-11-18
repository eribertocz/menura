//! `E0C-32`, `E0C-33` and `E0C-37`: the inputs of derived metrics, in E0.d (DER-5.1.2).
//!
//! `AGG(name)` names a model metric and `AGG(dataset.metric)` a dataset metric of the
//! development format, both compared under E0D-1.1 and E0D-1.3. A derived formula with a name
//! that names no metric is `E0C-32`, and one without `AGG(…)` is `E0C-33`; both reject the
//! model. `AGG(…)` in a model metric that is neither derived nor a context metric is `E0C-37`:
//! the metric is unusable (E0-4 table). In the development format, a model metric whose
//! `AGG(…)` terms all name dataset metrics is read as a derived metric (DER-2.2).
//!
//! Context metrics are left to the context extension, which resolves their base
//! ([`super::context`]). `AGG(…)` in a field or a dataset metric is `E0F-02`, reported by
//! E0.f.

mod output;

pub(crate) use output::{
    DerivedInput, DerivedMetric, DerivedMetrics, DerivedOrigin, DerivedRejection,
};

use std::collections::HashMap;

use semantic_compiler_contracts::types::audit_record::OssieFormat;
use semantic_compiler_model::expression::{Expression, Reference};

use super::walk::metric_aggregates;
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions};
use crate::names::Name;
use crate::outline::{DatasetOutline, ElementOutline, ModelOutline, outline};
use crate::pointer::{JsonPointer, position, resolve};
use crate::read::{Location, Node};
use crate::structure::{CompiledVariant, ElementKind, Structure};

/// Resolves the `AGG(…)` terms of every model metric of the semantic model that `structure`
/// selected in `document`, with the derived-metric declarations of `extensions`.
///
/// # Errors
///
/// Returns a [`DerivedRejection`] when any `E0C-32` or `E0C-33` diagnostic is found, with the
/// `E0C-37` warnings found with them.
pub fn resolve_derived_references(
    document: &Node,
    structure: &Structure<'_>,
    extensions: &Extensions,
) -> Result<DerivedMetrics, DerivedRejection> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let trees = metric_trees(structure);
    let metrics = Metrics::new(&model);
    let development = structure.format == OssieFormat::Development;
    let mut found = DerivedMetrics::default();
    let mut errors = Vec::new();
    for metric in &model.metrics {
        let Some(tree) = trees.get(&metric.pointer) else {
            continue;
        };
        if extensions.is_context_metric(&metric.pointer) {
            continue;
        }
        let terms = metric_aggregates(tree);
        let Some((origin, place)) = derivation(extensions, metric, &terms, development) else {
            if !terms.is_empty() {
                found.unusable.push(metric.pointer.clone());
                found.warnings.push(outside_derived(document, metric));
            }
            continue;
        };
        match derived_metric(&metrics, metric, &terms, origin, place) {
            Ok(derived) => found.metrics.push(derived),
            Err(diagnostic) => errors.push(diagnostic),
        }
    }
    if errors.is_empty() {
        return Ok(found);
    }
    errors.sort_by_cached_key(|diagnostic| {
        (position(document, &diagnostic.pointer), diagnostic.code)
    });
    Err(DerivedRejection {
        errors,
        warnings: found.warnings,
    })
}

/// Why `metric`, whose formula has the `AGG(…)` terms `terms`, is derived, and where the
/// diagnostics about its formula are located; `None` when it is not derived (DER-2.2,
/// DER-5.1.2). In the `development` format, a metric whose terms all name dataset metrics is
/// derived.
fn derivation(
    extensions: &Extensions,
    metric: &ElementOutline<'_>,
    terms: &[&Reference],
    development: bool,
) -> Option<(DerivedOrigin, (JsonPointer, Location))> {
    if let Some(declaration) = extensions.derived_declaration(&metric.pointer) {
        return Some((
            DerivedOrigin::Declared(declaration.on_division_by_zero),
            (declaration.data.clone(), declaration.location),
        ));
    }
    let dataset_metrics = !terms.is_empty() && terms.iter().all(|term| term.qualifier.is_some());
    (development && dataset_metrics).then(|| {
        (
            DerivedOrigin::Development,
            (
                metric.expression_pointer.clone(),
                metric.expression_location,
            ),
        )
    })
}

/// The metrics `AGG(…)` can name: the model metrics, and the dataset metrics of each dataset.
struct Metrics<'model, 'document> {
    model: HashMap<&'model Name, &'model ElementOutline<'document>>,
    datasets: HashMap<&'model Name, &'model DatasetOutline<'document>>,
}

impl<'model, 'document> Metrics<'model, 'document> {
    /// The first metric and dataset of each name in `model`.
    fn new(model: &'model ModelOutline<'document>) -> Self {
        let mut metrics = HashMap::new();
        for metric in &model.metrics {
            if let Some(name) = &metric.name {
                metrics.entry(&name.name).or_insert(metric);
            }
        }
        let mut datasets = HashMap::new();
        for dataset in &model.datasets {
            if let Some(name) = &dataset.name {
                datasets.entry(&name.name).or_insert(dataset);
            }
        }
        Self {
            model: metrics,
            datasets,
        }
    }

    /// The metric `AGG(reference)` names, if it exists.
    fn find(&self, reference: &Reference) -> Option<&'model ElementOutline<'document>> {
        let name = Name::of(&reference.name);
        match &reference.qualifier {
            None => self.model.get(&name).copied(),
            Some(dataset) => self
                .datasets
                .get(&Name::of(dataset))?
                .metrics
                .iter()
                .find(|metric| metric.name.as_ref().is_some_and(|found| found.name == name)),
        }
    }
}

/// The derived metric `metric` with the `AGG(…)` terms `terms`, or its `E0C-33` diagnostic
/// when it has none and its `E0C-32` diagnostic when a term names no metric, at `place`.
fn derived_metric(
    metrics: &Metrics<'_, '_>,
    metric: &ElementOutline<'_>,
    terms: &[&Reference],
    origin: DerivedOrigin,
    (pointer, location): (JsonPointer, Location),
) -> Result<DerivedMetric, ExtensionDiagnostic> {
    let diagnostic = |code, message| ExtensionDiagnostic {
        code,
        pointer: pointer.clone(),
        location,
        message,
    };
    if terms.is_empty() {
        return Err(diagnostic(
            ExtensionCode::NoInput,
            "the derived formula refers to no metric; it reads metrics with AGG(metric)".to_owned(),
        ));
    }
    let mut inputs = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for term in terms {
        if let Some(input) = metrics.find(term) {
            inputs.push(DerivedInput {
                reference: (*term).clone(),
                metric: input.pointer.clone(),
            });
        } else {
            let written = format!("`AGG({})`", written(term));
            if !missing.contains(&written) {
                missing.push(written);
            }
        }
    }
    if !missing.is_empty() {
        return Err(diagnostic(
            ExtensionCode::UnknownInput,
            format!(
                "the derived formula reads {}, which names no metric of the model",
                missing.join(", ")
            ),
        ));
    }
    Ok(DerivedMetric {
        name: metric
            .name
            .as_ref()
            .map_or_else(String::new, |name| name.written.text.to_owned()),
        pointer: metric.pointer.clone(),
        inputs,
        origin,
        diagnostic_pointer: pointer,
        diagnostic_location: location,
    })
}

/// The `E0C-37` diagnostic of `metric`, located at the metric, as every `W0G-01` is (E0G-2.8).
fn outside_derived(document: &Node, metric: &ElementOutline<'_>) -> ExtensionDiagnostic {
    ExtensionDiagnostic {
        code: ExtensionCode::AggregateOutsideDerived,
        pointer: metric.pointer.clone(),
        location: resolve(document, &metric.pointer)
            .map_or(metric.expression_location, |node| node.location),
        message: "the metric reads another metric with AGG(…), but it is marked neither `derived` nor `context`".to_owned(),
    }
}

/// A reference as written, such as `orders.total`.
fn written(reference: &Reference) -> String {
    match &reference.qualifier {
        Some(qualifier) => format!("{}.{}", qualifier.text(), reference.name.text()),
        None => reference.name.text().to_owned(),
    }
}

/// The tree of the compiled variant of every model metric that parses (E0B-3.3). A metric
/// whose expression cannot be used is reported by E0.b or E0.f.
pub(super) fn metric_trees<'structure>(
    structure: &'structure Structure<'_>,
) -> HashMap<&'structure JsonPointer, Expression> {
    structure
        .expressions
        .iter()
        .filter(|expression| expression.kind == ElementKind::Metric)
        .filter_map(|expression| {
            let text = match expression.variant {
                CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text) => text,
                CompiledVariant::Unusable => return None,
            };
            let tree = parse_element_expression(text, ExpressionPlace::Metric).ok()?;
            Some((&expression.pointer, tree))
        })
        .collect()
}
