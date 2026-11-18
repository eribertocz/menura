//! The derived and context metrics in E0.f: what derived formulas contain (`E0C-30`,
//! `E0C-31`, `E0C-35`, DER-5.1.3), the cycles of the graph of the metrics they read (`E0C-34`,
//! DER-5.1.4, CTX-5.1.5, E0G-5.1), the type of each formula (DER-5.1.3) and the result type of
//! each context metric (CTX-5.1.7).
//!
//! `AGG(m)` has the result type and nullability of `m`, and the formula is typed by E0F-5a
//! like any expression: a typing problem makes the metric unusable, reported as `W0F-01`. A
//! formula that reads an untyped metric gets no type: an unusable input makes the derived
//! metric unusable in E0.g (DER-5.1.5). A context metric has the result type of its base, a
//! 64-bit float for `share_of_total` (CTX-3a.3), and is always nullable; one whose base is
//! untyped gets no type, and E0.g makes it unusable (CTX-5.1.6).

mod formula;
mod graph;

use std::collections::HashMap;

use semantic_compiler_contracts::types::catalog::CatalogSchemaDocument;
use semantic_compiler_model::expression::Reference;

use super::elements::{ElementTypes, record};
use super::output::{ElementKind, TypedElement, Typing, UnusableElement};
use super::scope::Scope;
use super::stop::{Outcome, Stop};
use super::typer::{Aggregates, ExpressionTyper, MetricAggregates};
use super::value_type::{Typed, double};
use crate::extensions::context::Modifier;
use crate::extensions::{ExtensionCode, ExtensionDiagnostic};
use crate::names::Name;
use crate::outline::{ElementOutline, outline};
use crate::pointer::{JsonPointer, position};
use crate::read::Node;
use crate::references::context::{ContextMetric, ContextMetrics};
use crate::references::derived::{DerivedMetric, DerivedMetrics};
use crate::structure::Structure;
use formula::formula_problems;
use graph::{Dependent, search};

/// Checks and types the formula of every derived metric of `derived`, and types every context
/// metric of `context`, in the semantic model that `structure` selected in `document`, reading
/// the types `typing` found for the other metrics. `catalog` is the admitted catalog schema
/// document.
///
/// The metrics typed, the derived metrics a typing problem makes unusable and their `W0F-01`
/// warnings are in document order.
///
/// # Errors
///
/// Returns every `E0C-30`, `E0C-31` and `E0C-35` diagnostic, at most one per code and metric,
/// and one `E0C-34` diagnostic per cycle, in document order then by code. Each rejects the
/// model (E0-4 table).
pub fn type_derived_metrics(
    document: &Node,
    structure: &Structure<'_>,
    catalog: &CatalogSchemaDocument,
    typing: &Typing,
    derived: &DerivedMetrics,
    context: &ContextMetrics,
) -> Result<Typing, Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let scope = Scope::new(&model, catalog, &structure.expressions);
    let mut dependents: Vec<Dependent<'_>> = derived
        .metrics
        .iter()
        .map(Dependent::Derived)
        .chain(context.metrics.iter().map(Dependent::Context))
        .collect();
    dependents.sort_by_cached_key(|metric| position(document, metric.pointer()));
    let graph = search(&dependents);
    let errors = rejecting_diagnostics(document, &scope, derived, &graph.cycles);
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut found = type_in_order(&scope, typing, &graph.order);
    found
        .elements
        .sort_by_cached_key(|element| position(document, &element.pointer));
    found
        .unusable
        .sort_by_cached_key(|element| position(document, &element.pointer));
    found
        .warnings
        .sort_by_cached_key(|diagnostic| position(document, &diagnostic.pointer));
    Ok(found)
}

/// The `E0C-30`, `E0C-31` and `E0C-35` diagnostics of the formulas of `derived`, and one
/// `E0C-34` per cycle of `cycles`, in document order then by code.
fn rejecting_diagnostics(
    document: &Node,
    scope: &Scope<'_, '_>,
    derived: &DerivedMetrics,
    cycles: &[Vec<Dependent<'_>>],
) -> Vec<ExtensionDiagnostic> {
    let mut errors = Vec::new();
    for metric in &derived.metrics {
        if let Some(tree) = scope.tree(&metric.pointer) {
            for (code, message) in formula_problems(tree, metric) {
                errors.push(diagnostic(metric, code, message));
            }
        }
    }
    errors.extend(cycles.iter().filter_map(|cycle| cycle_diagnostic(cycle)));
    errors.sort_by_cached_key(|diagnostic| {
        (position(document, &diagnostic.pointer), diagnostic.code)
    });
    errors
}

/// Types the metrics of `order`, each after the dependent metrics it reads, with the types
/// `typing` found for the other metrics.
fn type_in_order(scope: &Scope<'_, '_>, typing: &Typing, order: &[Dependent<'_>]) -> Typing {
    let elements: HashMap<&JsonPointer, &ElementOutline<'_>> = scope
        .model
        .metrics
        .iter()
        .map(|metric| (&metric.pointer, metric))
        .collect();
    let mut known: HashMap<JsonPointer, Typed> = typing
        .elements
        .iter()
        .filter(|element| element.kind != ElementKind::Field)
        .map(|element| (element.pointer.clone(), element.typed.clone()))
        .collect();
    let mut found = Typing::default();
    for dependent in order {
        let metric = match dependent {
            Dependent::Derived(metric) => metric,
            Dependent::Context(metric) => {
                if let Some(typed) = context_type(&known, metric) {
                    known.insert(metric.pointer.clone(), typed.clone());
                    found.elements.push(TypedElement {
                        kind: ElementKind::Metric,
                        pointer: metric.pointer.clone(),
                        typed,
                    });
                }
                continue;
            }
        };
        let Some(element) = elements.get(&metric.pointer) else {
            continue;
        };
        let formula = type_formula(scope, &known, metric, element);
        if let Some(typed) = formula.types.get(&metric.pointer) {
            known.insert(metric.pointer.clone(), typed.clone());
            found.elements.push(TypedElement {
                kind: ElementKind::Metric,
                pointer: metric.pointer.clone(),
                typed: typed.clone(),
            });
        }
        for finding in formula.findings {
            found.unusable.push(UnusableElement {
                kind: ElementKind::Metric,
                pointer: metric.pointer.clone(),
                query_error: finding.diagnostic.code,
            });
            found.warnings.push(finding.diagnostic);
        }
    }
    found
}

/// Types the formula of `metric`, whose element is `element`, with the types `known` of the
/// metrics typed so far.
fn type_formula<'model>(
    scope: &Scope<'model, '_>,
    known: &HashMap<JsonPointer, Typed>,
    metric: &DerivedMetric,
    element: &'model ElementOutline<'_>,
) -> ElementTypes<'model> {
    let fields = |_: &Reference| -> Outcome<Typed> { Err(Stop::ReportedElsewhere) };
    let metrics = |reference: &Reference| input_type(known, metric, reference);
    let formula_typer = ExpressionTyper::new(&fields, Aggregates::Allowed)
        .with_metric_aggregates(MetricAggregates::Typed(&metrics));
    let mut found = ElementTypes::default();
    record(
        scope,
        &formula_typer,
        element,
        ElementKind::Metric,
        &mut found,
    );
    found
}

/// The type of `AGG(reference)` in the formula of `metric`: the type of the metric it names,
/// when that metric is typed (DER-5.1.3). An untyped metric is unusable, which E0.g reports
/// (DER-5.1.5).
fn input_type(
    known: &HashMap<JsonPointer, Typed>,
    metric: &DerivedMetric,
    reference: &Reference,
) -> Outcome<Typed> {
    let same = |left: &Reference| {
        Name::of(&left.name) == Name::of(&reference.name)
            && left.qualifier.as_ref().map(Name::of) == reference.qualifier.as_ref().map(Name::of)
    };
    metric
        .inputs
        .iter()
        .find(|input| same(&input.reference))
        .and_then(|input| known.get(&input.metric))
        .cloned()
        .ok_or(Stop::ReportedElsewhere)
}

/// The result type of the context metric `metric`, with the types `known` of the metrics typed
/// so far: that of its base, a 64-bit float for `share_of_total` (CTX-3a.3), and always
/// nullable (CTX-5.1.7). `None` when the base is untyped.
fn context_type(known: &HashMap<JsonPointer, Typed>, metric: &ContextMetric) -> Option<Typed> {
    let base = known.get(&metric.base)?;
    let mut typed = match metric.modifier {
        Modifier::ShareOfTotal(_) => Typed::value(double(), true),
        Modifier::PeriodToDate(_) | Modifier::PriorPeriod { .. } => base.clone(),
    };
    typed.nullable = true;
    Some(typed)
}

/// A diagnostic `code` about the formula of `metric`.
fn diagnostic(metric: &DerivedMetric, code: ExtensionCode, message: String) -> ExtensionDiagnostic {
    ExtensionDiagnostic {
        code,
        pointer: metric.diagnostic_pointer.clone(),
        location: metric.diagnostic_location,
        message,
    }
}

/// The `E0C-34` diagnostic of `cycle`, at its first metric (DER-5.1.4).
fn cycle_diagnostic(cycle: &[Dependent<'_>]) -> Option<ExtensionDiagnostic> {
    let (pointer, location) = cycle.first()?.diagnostic_place();
    let names: Vec<&str> = cycle.iter().map(|metric| metric.name()).collect();
    let metrics = if cycle
        .iter()
        .all(|metric| matches!(metric, Dependent::Derived(_)))
    {
        "derived metrics"
    } else {
        "derived and context metrics"
    };
    Some(ExtensionDiagnostic {
        code: ExtensionCode::DependencyCycle,
        pointer: pointer.clone(),
        location,
        message: format!("the {metrics} depend on each other: {}", names.join(" → ")),
    })
}
