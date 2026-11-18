//! Substage E0.f, typing: types every field and metric expression of the selected semantic
//! model against the catalog schema, and the relationship key pairs (E0F-1, E0F-3 to E0F-8,
//! E0E-3.7). The expression typed is the compiled variant E0.b selects: the single `ANSI_SQL`
//! variant, or the development-line `sql`; the dataset-scoped metrics of the development format
//! are typed too (section 4.5).
//!
//! Each expression gets a result type (E0F-5, E0F-5a, E0F-10) and whether it can be NULL
//! (E0F-7.1). A typing problem (`E0F-03` to `E0F-08`) makes its element unusable and is
//! reported as `W0F-01` (E0-4.1); typing never rejects the model.
//!
//! An expression that does not parse, a reference that does not resolve and the reference
//! forms of E0D-3.4 are reported by their own checks, never here: such an element gets no type
//! and no typing diagnostic. Expressions inside extensions are typed by each extension: the
//! conditions and time fields of populations by [`populations::type_populations`] (`E0C-23`,
//! `E0C-24`, POP-4.4, POP-4.5), and the `over` and population of additivity declarations by
//! [`additivity::type_additivity`] (`E0C-12`, `E0C-17`, ADD-5.1.5, ADD-5.1.7). The formulas of
//! derived metrics, the result types of context metrics and the graph of the metrics they read
//! are checked and typed by [`derived::type_derived_metrics`] (`E0C-30`, `E0C-31`, `E0C-34`,
//! `E0C-35`, DER-5.1.3, DER-5.1.4, CTX-5.1.5, CTX-5.1.7), and the kind of the time dimension of
//! context metrics by [`context::check_context_time_kinds`] (`E0C-61`, CTX-5.1.4). `AGG(…)` in
//! a field or a dataset metric is `E0F-02` (DER-5.1.2).

pub(crate) mod additivity;
mod aggregates;
mod arithmetic;
pub(crate) mod context;
mod conversion;
mod dataset_metrics;
mod dates;
pub(crate) mod derived;
mod diagnostic;
mod elements;
mod functions;
mod keys;
mod numbers;
mod operators;
mod output;
pub(crate) mod populations;
mod scope;
mod stop;
mod typer;
mod value_type;

pub(crate) use diagnostic::TypingCode;
pub(crate) use output::{ElementKind, TypedElement, Typing, UnusableElement};

use std::collections::HashMap;

use semantic_compiler_contracts::types::catalog::CatalogSchemaDocument;

use crate::outline::outline;
use crate::pointer::{JsonPointer, position};
use crate::read::Node;
use crate::references::derived::DerivedMetrics;
use crate::structure::Structure;
use dataset_metrics::type_dataset_metrics;
use elements::{ElementTypes, Finding, type_fields, type_metrics};
use scope::Scope;

/// Types the expressions of the semantic model that `structure` selected in `document`,
/// against `catalog`, the admitted catalog schema document. The formulas of the derived metrics
/// E0.d found in `derived` are left to [`derived::type_derived_metrics`] (DER-5.1.3).
///
/// Typed elements and unusable elements are in document order; diagnostics are in document
/// order, then by code (E0-4.3).
pub fn type_expressions(
    document: &Node,
    structure: &Structure<'_>,
    catalog: &CatalogSchemaDocument,
    derived: &DerivedMetrics,
) -> Typing {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let scope = Scope::new(&model, catalog, &structure.expressions);
    let fields = type_fields(&scope);
    let dataset_metrics = type_dataset_metrics(&scope, &fields.types);
    let metrics = type_metrics(&scope, &fields.types, derived);
    let mut findings = keys::key_findings(&scope, &fields.types);
    let mut elements = typed_elements(&fields, ElementKind::Field);
    elements.extend(typed_elements(&dataset_metrics, ElementKind::DatasetMetric));
    elements.extend(typed_elements(&metrics, ElementKind::Metric));
    findings.extend(fields.findings);
    findings.extend(dataset_metrics.findings);
    findings.extend(metrics.findings);
    elements.sort_by_cached_key(|element| position(document, &element.pointer));
    findings.sort_by_cached_key(|finding| {
        (
            position(document, &finding.diagnostic.pointer),
            finding.diagnostic.code,
        )
    });
    let mut unusable: HashMap<JsonPointer, UnusableElement> = HashMap::new();
    let mut warnings = Vec::new();
    for Finding {
        diagnostic,
        element: (kind, pointer),
    } in findings
    {
        unusable
            .entry(pointer.clone())
            .and_modify(|element| element.query_error = element.query_error.min(diagnostic.code))
            .or_insert(UnusableElement {
                kind,
                pointer,
                query_error: diagnostic.code,
            });
        warnings.push(diagnostic);
    }
    let mut unusable: Vec<UnusableElement> = unusable.into_values().collect();
    unusable.sort_by_cached_key(|element| position(document, &element.pointer));
    Typing {
        elements,
        unusable,
        warnings,
    }
}

fn typed_elements(types: &ElementTypes<'_>, kind: ElementKind) -> Vec<TypedElement> {
    types
        .types
        .iter()
        .map(|(pointer, typed)| TypedElement {
            kind,
            pointer: (*pointer).clone(),
            typed: typed.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests;
