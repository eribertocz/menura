//! Stage E0.b: validates the data tree against the official Apache Ossie 0.1.1 structure
//! (E0B-1, MDL-1.1), selects the semantic model to compile (E0B-3.2) and picks the single
//! `ANSI_SQL` variant of each expression (E0B-3.3, E0B-3.4).
//!
//! The structure is the official JSON Schema, embedded at compile time ([`schema::OFFICIAL_SCHEMA_SOURCE`])
//! and evaluated by the evaluator of `semantic-compiler-contracts`. `dialects`, `vendors` and
//! `ai_context` are validated and kept, and never affect compilation (E0B-3.5).

mod diagnostic;
mod json;
mod output;
mod schema;
mod selection;
mod variants;
mod violations;

use diagnostic::{StructureCode, StructureDiagnostic};
pub(crate) use output::{SelectedModel, Structure};

use crate::pointer::{JsonPointer, position};
use crate::read::{Node, Value};
use selection::select_model;
use variants::pick_variants;
use violations::official_schema_diagnostics;

/// Validates the structure of a model document read by [`crate::read::read_document`].
///
/// `model_name` is the load option of the same name (DEV-7). Every diagnostic is located by
/// JSON Pointer (E0B-3.6) and diagnostics are sorted in document order, then by code (E0-4.3).
///
/// # Errors
///
/// Returns every rejecting diagnostic, `E0B-01` to `E0B-07`, sorted; the list is never empty.
pub fn validate_structure<'document>(
    document: &'document Node,
    model_name: Option<&str>,
) -> Result<Structure<'document>, Vec<StructureDiagnostic>> {
    let mut errors = official_schema_diagnostics(document);
    let selected = match select_model(document, model_name) {
        Ok(selected) => selected,
        Err(diagnostic) => {
            errors.push(diagnostic);
            None
        }
    };
    if !errors.is_empty() {
        return Err(sorted(document, errors));
    }
    let Some(semantic_model) = selected.and_then(|index| model_at(document, index)) else {
        // Unreachable: without a list of semantic models the schema diagnostics are not empty.
        return Err(vec![StructureDiagnostic {
            code: StructureCode::MissingProperty,
            pointer: JsonPointer::root().member("semantic_model"),
            location: document.location,
            message: "the document has no list of semantic models".to_owned(),
        }]);
    };
    let found = pick_variants(semantic_model.node, &semantic_model.pointer);
    let mut expressions = found.expressions;
    expressions.sort_by_cached_key(|expression| position(document, &expression.pointer));
    Ok(Structure {
        semantic_model,
        expressions,
        warnings: sorted(document, found.warnings),
    })
}

/// The semantic model at `index` of `semantic_model`.
fn model_at(document: &Node, index: usize) -> Option<SelectedModel<'_>> {
    let list = selection::member(document, "semantic_model")?;
    let Value::Sequence(models) = &list.value else {
        return None;
    };
    Some(SelectedModel {
        index,
        pointer: JsonPointer::root().member("semantic_model").item(index),
        node: models.get(index)?,
    })
}

/// Sorts diagnostics in document order then by code, without repeating a code at one node.
fn sorted(document: &Node, mut diagnostics: Vec<StructureDiagnostic>) -> Vec<StructureDiagnostic> {
    diagnostics.sort_by_cached_key(|diagnostic| {
        (position(document, &diagnostic.pointer), diagnostic.code)
    });
    diagnostics
        .dedup_by(|later, earlier| later.pointer == earlier.pointer && later.code == earlier.code);
    diagnostics
}

#[cfg(test)]
mod tests;
