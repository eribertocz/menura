//! Stage E0.d: resolves the names and references of the selected semantic model against the
//! model itself and the catalog schema (E0D-1 to E0D-4, DEV-2, DEV-3).
//!
//! Names are compared after normalization (E0D-1). Duplicate and over-long names (`E0D-01`) and
//! sources the catalog does not describe (`E0D-02`) reject the model; a missing reference
//! (`E0D-03` to `E0D-07`) makes its element unusable and is reported as `W0D-01` (E0-4).
//!
//! A query `source` that is not a single read-only `SELECT` (`E0D-08`) rejects the model too,
//! and a reference form E0D-3 does not list is `E0F-02`, reported as `W0F-01` (E0D-3.4).
//!
//! References inside extensions are resolved by each extension with its own codes (E0D-3.2),
//! never by the generic checks here: the fields of populations by
//! [`populations::resolve_population_references`] (`E0C-22`, POP-4.3), and the inputs of
//! derived metrics by [`derived::resolve_derived_references`] (`E0C-32`, `E0C-33`, `E0C-37`,
//! DER-5.1.2), and the base and removed fields of context metrics by
//! [`context::resolve_context_references`] (`E0C-60`, `E0C-63`, `E0C-64`, CTX-5.1.3), and the
//! fields and roles of `access.fields` by [`security::resolve_security_references`]
//! (`E0C-49`, SCY-5.1.4).

pub(crate) mod context;
mod dataset_metric_references;
pub(crate) mod derived;
mod diagnostic;
mod expression_references;
mod finding;
mod keys;
mod output;
pub(crate) mod populations;
mod query_sources;
mod scope;
pub(crate) mod security;
mod sources;
pub(crate) mod summaries;
mod uniqueness;
mod walk;

pub(crate) use diagnostic::ReferenceCode;
pub(crate) use output::{ElementKind, ReferenceRejection, References, UnusableElement};

use std::collections::HashMap;

use semantic_compiler_contracts::types::catalog::CatalogSchemaDocument;

use crate::extensions::summaries::SummaryDatasets;
use crate::outline::{self, outline};
use crate::pointer::{JsonPointer, position};
use crate::read::Node;
use crate::structure::Structure;
use finding::Finding;
use scope::Scope;

/// Resolves the references of the semantic model that `structure` selected in `document`,
/// against `catalog`, the admitted catalog schema document.
///
/// Diagnostics are sorted in document order, then by code (E0-4.3).
///
/// # Errors
///
/// Returns a [`ReferenceRejection`] when any `E0D-01`, `E0D-02` or `E0D-08` diagnostic is
/// found, with the warnings found with them.
pub fn resolve_references(
    document: &Node,
    structure: &Structure<'_>,
    catalog: &CatalogSchemaDocument,
    summaries: &SummaryDatasets,
) -> Result<References, ReferenceRejection> {
    let _ = summaries;
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let scope = Scope::new(&model, catalog, &structure.expressions);
    let semantic_models = outline::semantic_model_names(document);
    let mut findings = uniqueness::name_findings(&semantic_models, &model);
    findings.extend(sources::source_findings(&scope));
    findings.extend(query_sources::query_source_findings(&scope));
    findings.extend(expression_references::column_findings(&scope));
    findings.extend(expression_references::metric_findings(&scope));
    findings.extend(expression_references::form_findings(&scope));
    findings.extend(dataset_metric_references::dataset_metric_findings(&scope));
    findings.extend(keys::dataset_key_findings(&scope));
    findings.extend(keys::relationship_findings(&scope));
    findings.sort_by_cached_key(|finding| {
        (
            position(document, &finding.diagnostic.pointer),
            finding.diagnostic.code,
        )
    });
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut unusable: HashMap<JsonPointer, UnusableElement> = HashMap::new();
    for Finding {
        diagnostic,
        element,
    } in findings
    {
        match element {
            None => errors.push(diagnostic),
            Some((kind, pointer)) => {
                mark_unusable(&mut unusable, kind, pointer, diagnostic.code);
                warnings.push(diagnostic);
            }
        }
    }
    if !errors.is_empty() {
        return Err(ReferenceRejection { errors, warnings });
    }
    let mut unusable: Vec<UnusableElement> = unusable.into_values().collect();
    unusable.sort_by_cached_key(|element| position(document, &element.pointer));
    Ok(References { unusable, warnings })
}

/// Records that `pointer` is unusable because of `code`, keeping the lowest code of an element
/// (E2-5 row 12a).
fn mark_unusable(
    unusable: &mut HashMap<JsonPointer, UnusableElement>,
    kind: ElementKind,
    pointer: JsonPointer,
    code: ReferenceCode,
) {
    unusable
        .entry(pointer.clone())
        .and_modify(|element| element.query_error = element.query_error.min(code))
        .or_insert(UnusableElement {
            kind,
            pointer,
            query_error: code,
        });
}

#[cfg(test)]
mod tests;
