//! Substage E0.g: classifies the metrics of the selected semantic model (E0G-1 to E0G-6): base
//! model metrics, the dataset-scoped metrics of the development format, and derived metrics.
//!
//! Each metric usable after E0.f is split into components, one per aggregate, each with its
//! home dataset and decomposability class; its additivity and recomposability are deduced from
//! the classes. A metric that breaks E0G-2.1, E0G-2.3 to E0G-2.5 or E0G-2.7 is unusable and is
//! reported once as `W0G-01`, at the metric (E0G-2.8); classification never rejects the model.
//!
//! Window functions (E0G-2.2) and nested aggregates (E0G-2.6) make the metric unusable in
//! E0.f, which reports them; such a metric is not classified. The additivity declaration E0.c
//! validated replaces the deduced additivity, and one that relaxes it or is semi-additive over
//! several home datasets makes the metric unusable as `E0G-01` (E0G-4.1 to E0G-4.3).
//!
//! Derived metrics are not base metrics, and E0G-2 does not apply to them: each is classified
//! from its inputs (E0G-5.2, E0G-5.3, DER-5.1.6), and is unusable when one of them is
//! (DER-5.1.5). Context metrics are not base metrics either: each is classified from its base,
//! and is unusable when its base is (CTX-5.1.6, CTX-5.1.7).

mod additivity;
mod context;
mod decomposability;
mod derived;
mod diagnostic;
mod input;
mod output;
mod parts;
mod rules;

pub(crate) use diagnostic::{ClassificationCode, ClassificationDiagnostic};
pub(crate) use input::ClassificationInput;
pub(crate) use output::{
    Classification, ClassifiedMetric, Component, DecomposabilityClass, MetricKind, UnusableMetric,
};

use std::collections::{BTreeSet, HashMap};

use semantic_compiler_model::expression::Expression;

use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::additivity::DeclaredAdditivity;
use crate::outline::{ElementOutline, ModelOutline, outline};
use crate::pointer::{JsonPointer, position, resolve};
use crate::read::Node;
use crate::structure::{CompiledVariant, ElementExpression, ElementKind, Structure};
use crate::typing::{self, TypedElement};
use additivity::additivity;
use decomposability::is_recomposable;
use derived::classify_derived;
pub(crate) use output::Additivity;
use parts::expression_parts;
use rules::{MetricScope, Problem, components};

/// Classifies the metrics of the semantic model that `structure` selected in `document`,
/// reading what the substages before E0.g found in `input`: the types E0.f found, the
/// additivity declarations (E0G-4.3), the derived metrics and their inputs, the context metrics
/// and their bases, and the query error of every metric made unusable before E0.g (DER-5.1.5,
/// CTX-5.1.6).
///
/// Only the base metrics E0.f typed are classified: the others are unusable after E0.f
/// (E0G-2.8). Classified and unusable metrics, and diagnostics, are in document order (E0-4.3).
pub fn classify_metrics(
    document: &Node,
    structure: &Structure<'_>,
    input: &ClassificationInput<'_>,
) -> Classification {
    let (typing, extensions) = (input.typing, input.extensions);
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let trees: HashMap<&JsonPointer, Expression> = structure
        .expressions
        .iter()
        .filter_map(metric_tree)
        .collect();
    let typed: HashMap<&JsonPointer, &TypedElement> = typing
        .elements
        .iter()
        .filter(|element| element.kind != typing::ElementKind::Field)
        .map(|element| (&element.pointer, element))
        .collect();
    let declarations: HashMap<&JsonPointer, &DeclaredAdditivity> = extensions
        .additivity
        .iter()
        .map(|declaration| (&declaration.metric, &declaration.additivity))
        .collect();
    let mut classification = Classification::default();
    for (metric, kind, scope) in base_metrics(&model, input) {
        let (Some(tree), Some(typed)) = (trees.get(&metric.pointer), typed.get(&metric.pointer))
        else {
            continue;
        };
        let declaration = declarations.get(&metric.pointer).copied();
        let found = components(&expression_parts(tree), scope).and_then(|components| {
            let additivity = additivity(&components, declaration)?;
            Ok((components, additivity))
        });
        match found {
            Ok((components, additivity)) => {
                let parts = ClassifiedParts {
                    components,
                    additivity,
                    declaration: declaration.cloned(),
                };
                if let Some(classified) = classified(metric, kind, parts, typed) {
                    classification.metrics.push(classified);
                }
            }
            Err(problem) => record_problem(document, metric, kind, problem, &mut classification),
        }
    }
    classify_derived(document, structure, input, &mut classification);
    sort_in_document_order(document, &mut classification);
    classification
}

/// Every base metric of `model`, with its kind and scope: the dataset metrics of each dataset,
/// then the model metrics. A dataset metric of a dataset without a name is not a metric of any
/// dataset, and is left out; so are the derived metrics, the metrics `E0C-37` makes unusable,
/// and the context metrics (DER-5.1.6, CTX-5.1.7).
fn base_metrics<'model, 'document>(
    model: &'model ModelOutline<'document>,
    input: &ClassificationInput<'_>,
) -> Vec<(
    &'model ElementOutline<'document>,
    MetricKind,
    MetricScope<'model>,
)> {
    let mut metrics = Vec::new();
    for dataset in &model.datasets {
        let Some(name) = &dataset.name else {
            continue;
        };
        for metric in &dataset.metrics {
            metrics.push((
                metric,
                MetricKind::DatasetMetric,
                MetricScope::Dataset(&name.name),
            ));
        }
    }
    for metric in &model.metrics {
        let pointer = &metric.pointer;
        let other = input.derived.metric(pointer).is_some()
            || input.derived.unusable.contains(pointer)
            || input.extensions.is_context_metric(pointer);
        if !other {
            metrics.push((metric, MetricKind::Metric, MetricScope::Model));
        }
    }
    metrics
}

/// Sorts the metrics and diagnostics of `classification` in document order (E0-4.3).
fn sort_in_document_order(document: &Node, classification: &mut Classification) {
    classification
        .metrics
        .sort_by_cached_key(|metric| position(document, &metric.pointer));
    classification
        .unusable
        .sort_by_cached_key(|metric| position(document, &metric.pointer));
    classification
        .warnings
        .sort_by_cached_key(|diagnostic| position(document, &diagnostic.pointer));
}

/// The tree of the compiled variant of a model metric or dataset metric, if it parses. An
/// expression that does not parse is reported by E0.f.
fn metric_tree<'structure>(
    expression: &'structure ElementExpression<'_>,
) -> Option<(&'structure JsonPointer, Expression)> {
    if expression.kind == ElementKind::Field {
        return None;
    }
    let text = match expression.variant {
        CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text) => text,
        CompiledVariant::Unusable => return None,
    };
    let tree = parse_element_expression(text, ExpressionPlace::Metric).ok()?;
    Some((&expression.pointer, tree))
}

/// What classification found for a metric that breaks no rule.
struct ClassifiedParts {
    components: Vec<Component>,
    additivity: Additivity,
    declaration: Option<DeclaredAdditivity>,
}

/// The classified metric `metric` of `parts`, with the type E0.f found (E0G-6).
///
/// E0.f types no metric as the literal `NULL` (E0F-3.2), so a typed metric always has a
/// logical type.
fn classified(
    metric: &ElementOutline<'_>,
    kind: MetricKind,
    parts: ClassifiedParts,
    typed: &TypedElement,
) -> Option<ClassifiedMetric> {
    let result_type = typed.typed.logical()?.clone();
    let ClassifiedParts {
        components,
        additivity,
        declaration,
    } = parts;
    let recomposable = components
        .iter()
        .all(|component| is_recomposable(&component.call));
    let home_datasets: BTreeSet<_> = components
        .iter()
        .map(|component| component.home_dataset.clone())
        .collect();
    Some(ClassifiedMetric {
        kind,
        pointer: metric.pointer.clone(),
        inputs: Vec::new(),
        home_datasets,
        additivity,
        declaration,
        recomposable,
        components,
        result_type,
        nullable: typed.typed.nullable,
    })
}

/// Records that `metric` is unusable because of `problem`, with its `W0G-01` at the metric
/// (E0G-2.8).
fn record_problem(
    document: &Node,
    metric: &ElementOutline<'_>,
    kind: MetricKind,
    problem: Problem,
    classification: &mut Classification,
) {
    let location =
        resolve(document, &metric.pointer).map_or(metric.expression_location, |node| node.location);
    classification.unusable.push(UnusableMetric {
        kind,
        pointer: metric.pointer.clone(),
        query_error: problem.code.into(),
    });
    classification.warnings.push(ClassificationDiagnostic {
        code: problem.code.into(),
        pointer: metric.pointer.clone(),
        location,
        message: problem.message,
    });
}

#[cfg(test)]
mod tests;
