//! Stage E0.b: validates the data tree against the structure of its format (E0B-1, MDL-1.1,
//! OSS-5.1), selects the semantic model to compile (E0B-3.2), picks the compiled variant of each
//! expression (E0B-3.3, E0B-3.4, section 4.5), maps development-line `datatype` values to
//! logical kinds (OSS-5.2) and reads `nonportable_extensions` (MDL-2.4).
//!
//! The structure is the official 0.1.1 JSON Schema ([`schema::OFFICIAL_SCHEMA_SOURCE`]) or, with
//! `ossie_format: development`, the development-line JSON Schema at the tracked commit
//! ([`schema::DEVELOPMENT_SCHEMA_SOURCE`]). Both are embedded at compile time and evaluated by
//! the evaluator of `semantic-compiler-contracts`. `dialects`, `vendors` and `ai_context` are
//! validated and kept, and never affect compilation (E0B-3.5).

mod datatype;
mod diagnostic;
mod evaluable_schema;
mod json;
mod nonportable;
mod output;
mod schema;
mod selection;
mod variants;
mod violations;

use semantic_compiler_contracts::types::audit_record::OssieFormat;

use diagnostic::{StructureCode, StructureDiagnostic};
use output::{SelectedModel, Structure};

use crate::pointer::{JsonPointer, position};
use crate::read::{Node, Value};
use nonportable::nonportable_extensions;
use selection::{check_root_model, select_model};
use variants::pick_variants;
use violations::schema_diagnostics;

/// Validates the structure of a model document read by [`crate::read::read_document`].
///
/// `format` and `model_name` are the load options `ossie_format` (MDL-1.1) and `model_name`
/// (DEV-7). Every diagnostic is located by JSON Pointer (E0B-3.6) and diagnostics are sorted in
/// document order, then by code (E0-4.3).
///
/// # Errors
///
/// Returns every rejecting diagnostic, `E0B-01` to `E0B-07`, sorted; the list is never empty.
pub fn validate_structure<'document>(
    document: &'document Node,
    format: OssieFormat,
    model_name: Option<&str>,
) -> Result<Structure<'document>, Vec<StructureDiagnostic>> {
    let mut errors = schema_diagnostics(document, format);
    let selected = match format {
        OssieFormat::Release => select_model(document, model_name),
        OssieFormat::Development => check_root_model(document, model_name).map(|()| Some(0)),
    };
    let selected = match selected {
        Ok(selected) => selected,
        Err(diagnostic) => {
            errors.push(diagnostic);
            None
        }
    };
    if !errors.is_empty() {
        return Err(sorted(document, errors));
    }
    let Some(semantic_model) = selected.and_then(|index| model_at(document, format, index)) else {
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
    let mut extensions = nonportable_extensions(semantic_model.node, &semantic_model.pointer);
    extensions.sort_by_cached_key(|entry| position(document, &entry.pointer));
    Ok(Structure {
        format,
        semantic_model,
        expressions,
        nonportable_extensions: extensions,
        warnings: sorted(document, found.warnings),
    })
}

/// The semantic model at `index`: of `semantic_model` in the release format, or the root, the
/// one semantic model of the development format (section 4.5).
fn model_at(document: &Node, format: OssieFormat, index: usize) -> Option<SelectedModel<'_>> {
    if format == OssieFormat::Development {
        return Some(SelectedModel {
            index,
            pointer: JsonPointer::root(),
            node: document,
        });
    }
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
mod development_tests;
#[cfg(test)]
mod nonportable_tests;
#[cfg(test)]
mod tests;
