//! Selection of the semantic model to compile with the load option `model_name` (E0B-3.2,
//! DEV-7).

use semantic_compiler_contracts::normalize_unquoted_identifier;

use super::diagnostic::{StructureCode, StructureDiagnostic};
use crate::names::is_malformed;
use crate::nodes::member;
use crate::pointer::JsonPointer;
use crate::read::{Node, Value};

/// The index of the semantic model to compile, or the `E0B-07` diagnostic.
///
/// Returns `Ok(None)` when `semantic_model` is not a list: the schema diagnostics already reject
/// the document. A document without any semantic model is rejected, whether or not
/// `model_name` is given (E0B-3.2). With several semantic models, `model_name` must name one;
/// with one, a `model_name` that is given must name it. A `model_name` that names more than one
/// semantic model is rejected too (`E0B-07`). `model_name` and each `name` are read as declared
/// names (E0D-1.4) and match when they are the same name (E0D-1.1, E0D-1.3); a malformed
/// delimited `model_name` (E0D-1.4) names no semantic model.
pub(crate) fn select_model(
    document: &Node,
    model_name: Option<&str>,
) -> Result<Option<usize>, StructureDiagnostic> {
    let pointer = JsonPointer::root().member("semantic_model");
    let Some(list) = member(document, "semantic_model") else {
        return Ok(None);
    };
    let Value::Sequence(models) = &list.value else {
        return Ok(None);
    };
    let rejection = |message: String| StructureDiagnostic {
        code: StructureCode::ModelSelection,
        pointer: pointer.clone(),
        location: list.location,
        message,
    };
    if models.is_empty() {
        return Err(rejection(
            "the document has no semantic model to load".to_owned(),
        ));
    }
    match model_name {
        Some(name) => match named_models(models, name).as_slice() {
            [index] => Ok(Some(*index)),
            [] => Err(rejection(format!(
                "`model_name` `{name}` names no semantic model"
            ))),
            several => Err(rejection(format!(
                "`model_name` `{name}` names {} semantic models",
                several.len()
            ))),
        },
        None if models.len() == 1 => Ok(Some(0)),
        None => Err(rejection(format!(
            "the document has {} semantic models; the load option `model_name` must name one",
            models.len()
        ))),
    }
}

/// The `E0B-07` diagnostic of a development-line document whose `name` is not `model_name`.
///
/// The flat root is the one semantic model (section 4.5), so a `model_name` that is given must
/// name it (E0B-3.2). `model_name` and `name` are compared as in [`select_model`], as declared
/// names (E0D-1.4) that match when they are the same name (E0D-1.1, E0D-1.3), so `shop` names a
/// model named `SHOP`. A malformed delimited `model_name` (E0D-1.4) names no model. A missing
/// or non-text `name` is left to the schema diagnostics.
pub(crate) fn check_root_model(
    document: &Node,
    model_name: Option<&str>,
) -> Result<(), StructureDiagnostic> {
    let (Some(name), Some(node)) = (model_name, member(document, "name")) else {
        return Ok(());
    };
    let Value::String(root_name) = &node.value else {
        return Ok(());
    };
    if !is_malformed(name) && declared_name_key(root_name) == declared_name_key(name) {
        return Ok(());
    }
    Err(StructureDiagnostic {
        code: StructureCode::ModelSelection,
        pointer: JsonPointer::root(),
        location: document.location,
        message: format!("`model_name` `{name}` does not name the semantic model"),
    })
}

/// The indexes of the semantic models whose `name` is the same name as `name` (E0D-1.3): none
/// when `name` is a malformed delimited name (E0D-1.4).
fn named_models(models: &[Node], name: &str) -> Vec<usize> {
    if is_malformed(name) {
        return Vec::new();
    }
    let wanted = declared_name_key(name);
    models
        .iter()
        .enumerate()
        .filter(|(_, model)| {
            member(model, "name").is_some_and(|node| {
                matches!(&node.value, Value::String(text) if declared_name_key(text) == wanted)
            })
        })
        .map(|(index, _)| index)
        .collect()
}

/// The form in which two declared names are compared (E0D-1.4).
///
/// A delimited name, whose text starts and ends with `"`, is its text without the quotes and
/// with `""` read as `"`. A regular name is normalized to upper case (E0D-1.1).
fn declared_name_key(text: &str) -> String {
    match text
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        Some(delimited) => delimited.replace("\"\"", "\""),
        None => normalize_unquoted_identifier(text),
    }
}
