//! Substage E0.f, typing: types every field and metric expression of the selected semantic
//! model against the catalog schema, and the relationship key pairs (E0F-1, E0F-3 to E0F-8,
//! E0E-3.7).
//!
//! Each expression gets a result type (E0F-5, E0F-5a, E0F-10) and whether it can be NULL
//! (E0F-7.1). A typing problem (`E0F-03` to `E0F-08`) makes its element unusable and is
//! reported as `W0F-01` (E0-4.1); typing never rejects the model.
//!
//! An expression that does not parse, a reference that does not resolve and the reference
//! forms of E0D-3.4 are reported by their own checks, never here: such an element gets no type
//! and no typing diagnostic. Expressions inside extensions are typed by each extension.
//! The cases issue #126 asks about get no type and no diagnostic until it is settled.

mod aggregates;
mod arithmetic;
mod conversion;
mod dates;
mod diagnostic;
mod elements;
mod functions;
mod keys;
mod numbers;
mod operators;
mod output;
mod scope;
mod stop;
mod typer;
mod value_type;

pub(crate) use output::{ElementKind, TypedElement, Typing, UnusableElement};

use std::collections::HashMap;

use semantic_compiler_contracts::types::catalog::CatalogSchemaDocument;

use crate::outline::outline;
use crate::pointer::{JsonPointer, position};
use crate::read::Node;
use crate::structure::Structure;
use elements::{ElementTypes, Finding, type_fields, type_metrics};
use scope::Scope;

/// Types the expressions of the semantic model that `structure` selected in `document`,
/// against `catalog`, the admitted catalog schema document.
///
/// Typed elements and unusable elements are in document order; diagnostics are in document
/// order, then by code (E0-4.3).
pub fn type_expressions(
    document: &Node,
    structure: &Structure<'_>,
    catalog: &CatalogSchemaDocument,
) -> Typing {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let scope = Scope::new(&model, catalog, &structure.expressions);
    let fields = type_fields(&scope);
    let metrics = type_metrics(&scope, &fields.types);
    let mut findings = keys::key_findings(&scope, &fields.types);
    let mut elements = typed_elements(&fields, ElementKind::Field);
    elements.extend(typed_elements(&metrics, ElementKind::Metric));
    findings.extend(fields.findings);
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
