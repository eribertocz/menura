//! Selection of the semantic model to compile with the load option `model_name` (E0B-3.2,
//! DEV-7).

use super::diagnostic::{StructureCode, StructureDiagnostic};
use crate::nodes::{is_text, member};
use crate::pointer::JsonPointer;
use crate::read::{Node, Value};

/// The index of the semantic model to compile, or the `E0B-07` diagnostic.
///
/// Returns `Ok(None)` when `semantic_model` is not a list: the schema diagnostics already reject
/// the document. A document without any semantic model is rejected, whether or not
/// `model_name` is given (E0B-3.2). With several semantic models, `model_name` must name one;
/// with one, a `model_name` that is given must name it. When two semantic models have the
/// name, the first is selected: name uniqueness is checked in E0.d.
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
        Some(name) => models
            .iter()
            .position(|model| member(model, "name").is_some_and(|node| is_text(node, name)))
            .map(Some)
            .ok_or_else(|| rejection(format!("`model_name` `{name}` names no semantic model"))),
        None if models.len() == 1 => Ok(Some(0)),
        None => Err(rejection(format!(
            "the document has {} semantic models; the load option `model_name` must name one",
            models.len()
        ))),
    }
}
