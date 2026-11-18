//! `E0C-60`, `E0C-63` and `E0C-64`: the base and the removed fields of context metrics, in
//! E0.d (CTX-5.1.3).
//!
//! `base` names a model metric, compared under E0D-1.1 and E0D-1.3, and the metric's
//! expression is exactly one `AGG(…)` term that names it. A base that names no model metric,
//! an expression of another form, and a metric that is both context and derived are `E0C-60`;
//! a base that is itself a context metric is `E0C-64`; a `remove` field whose dataset or field
//! does not exist is `E0C-63`. Each rejects the model (E0-4 table), and is located at the
//! `data` member of the metric's entry of ours (E0C-2.6).

mod output;

pub(crate) use output::{ContextMetric, ContextMetrics};

use std::collections::HashMap;

use semantic_compiler_model::expression::{Expression, Reference};

use super::derived::metric_trees;
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::extensions::context::ContextDeclaration;
use crate::extensions::{ExtensionCode, ExtensionDiagnostic, Extensions, Problems, report_at};
use crate::names::Name;
use crate::outline::{DatasetOutline, DeclaredName, ElementOutline, ModelOutline, outline};
use crate::pointer::{JsonPointer, position};
use crate::read::Node;
use crate::structure::Structure;

/// What resolving context metrics reads: the model metrics and datasets by name, and the tree
/// of every model metric that parses.
struct Model<'model, 'document> {
    metrics: HashMap<&'model Name, &'model ElementOutline<'document>>,
    datasets: HashMap<&'model Name, &'model DatasetOutline<'document>>,
    trees: HashMap<&'model JsonPointer, Expression>,
}

/// Resolves the base and the removed fields of every context declaration of `extensions`, in
/// the semantic model that `structure` selected in `document`.
///
/// # Errors
///
/// Returns every `E0C-60`, `E0C-63` and `E0C-64` diagnostic, at most one per code and
/// metric, in document order then by code.
pub fn resolve_context_references(
    document: &Node,
    structure: &Structure<'_>,
    extensions: &Extensions,
) -> Result<ContextMetrics, Vec<ExtensionDiagnostic>> {
    let outline = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let trees = metric_trees(structure);
    let model = Model {
        metrics: first_by_name(outline.metrics.iter().map(|metric| (&metric.name, metric))),
        datasets: first_by_name(
            outline
                .datasets
                .iter()
                .map(|dataset| (&dataset.name, dataset)),
        ),
        trees,
    };
    let mut found = ContextMetrics::default();
    let mut errors = Vec::new();
    for declaration in &extensions.context {
        let Some(metric) = metric_of(&outline, declaration) else {
            continue;
        };
        match context_metric(&model, extensions, metric, declaration) {
            Ok(context) => found.metrics.push(context),
            Err(diagnostics) => errors.extend(diagnostics),
        }
    }
    if errors.is_empty() {
        return Ok(found);
    }
    errors.sort_by_cached_key(|diagnostic| {
        (position(document, &diagnostic.pointer), diagnostic.code)
    });
    Err(errors)
}

/// The first element of each name of `named`, compared under E0D-1.3.
fn first_by_name<'model, Element>(
    named: impl Iterator<Item = (&'model Option<DeclaredName<'model>>, Element)>,
) -> HashMap<&'model Name, Element> {
    let mut found = HashMap::new();
    for (name, element) in named {
        if let Some(name) = name {
            found.entry(&name.name).or_insert(element);
        }
    }
    found
}

/// The model metric that declares `declaration`.
fn metric_of<'model, 'document>(
    outline: &'model ModelOutline<'document>,
    declaration: &ContextDeclaration,
) -> Option<&'model ElementOutline<'document>> {
    outline
        .metrics
        .iter()
        .find(|metric| metric.pointer == declaration.metric)
}

/// The context metric `metric` declared by `declaration`, or its diagnostics.
fn context_metric(
    model: &Model<'_, '_>,
    extensions: &Extensions,
    metric: &ElementOutline<'_>,
    declaration: &ContextDeclaration,
) -> Result<ContextMetric, Vec<ExtensionDiagnostic>> {
    let mut problems = Problems::new();
    if extensions.derived_declaration(&metric.pointer).is_some() {
        problems.push((
            ExtensionCode::ContextBase,
            "the metric is marked both `context` and `derived`; it is one or the other".to_owned(),
        ));
    }
    let base_name = base_name(&declaration.base);
    let base = base_name
        .as_ref()
        .and_then(|name| model.metrics.get(name).copied());
    match (&base_name, base) {
        (Some(name), Some(_)) => {
            if let Some(problem) = expression_problem(model, metric, name, &declaration.base) {
                problems.push((ExtensionCode::ContextBase, problem));
            }
        }
        _ => problems.push((
            ExtensionCode::ContextBase,
            format!("`base` `{}` names no metric of the model", declaration.base),
        )),
    }
    if let Some(base) = base
        && extensions.is_context_metric(&base.pointer)
    {
        problems.push((
            ExtensionCode::NestedContext,
            format!(
                "`base` `{}` is itself a context metric; a context metric has one level",
                declaration.base
            ),
        ));
    }
    problems.extend(removed_field_problems(model, declaration));
    match (base, problems.is_empty()) {
        (Some(base), true) => Ok(ContextMetric {
            pointer: metric.pointer.clone(),
            name: metric
                .name
                .as_ref()
                .map_or_else(String::new, |name| name.written.text.to_owned()),
            base: base.pointer.clone(),
            modifier: declaration.modifier.clone(),
            diagnostic_pointer: declaration.data.clone(),
            diagnostic_location: declaration.location,
        }),
        _ => Err(report_at(&declaration.data, declaration.location, problems)),
    }
}

/// The name `base` writes, read with the identifier rules of E0F-9.3: a regular or a quoted
/// identifier. Any other text names no metric.
fn base_name(base: &str) -> Option<Name> {
    match parse_element_expression(base, ExpressionPlace::Extension) {
        Ok(Expression::Reference(Reference {
            qualifier: None,
            name,
        })) => Some(Name::of(&name)),
        _ => None,
    }
}

/// Why the expression of `metric` is not `AGG(<base>)`, when its expression parses; one that
/// does not parse is reported by E0.f.
fn expression_problem(
    model: &Model<'_, '_>,
    metric: &ElementOutline<'_>,
    base: &Name,
    written: &str,
) -> Option<String> {
    let tree = model.trees.get(&metric.pointer)?;
    let names_base = matches!(
        tree,
        Expression::MetricAggregate(Reference { qualifier: None, name }) if Name::of(name) == *base
    );
    (!names_base).then(|| {
        format!(
            "the expression of a context metric must be `AGG({written})`, the AGG() of its base"
        )
    })
}

/// The `E0C-63` problem of the `remove` fields of `declaration` that do not exist.
fn removed_field_problems(
    model: &Model<'_, '_>,
    declaration: &ContextDeclaration,
) -> Option<(ExtensionCode, String)> {
    let missing: Vec<String> = declaration
        .modifier
        .removed_fields()
        .iter()
        .filter(|field| {
            model
                .datasets
                .get(&field.dataset)
                .and_then(|dataset| dataset.field(&field.field))
                .is_none()
        })
        .map(|field| format!("`{}`", field.written))
        .collect();
    (!missing.is_empty()).then(|| {
        (
            ExtensionCode::RemovedFieldMissing,
            format!(
                "`remove` names fields that do not exist: {}",
                missing.join(", ")
            ),
        )
    })
}
