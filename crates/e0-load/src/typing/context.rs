//! `E0C-61`, in E0.f: the `time_dimension` of a context metric has kind `date` or
//! `timestamp` (CTX-5.1.4). It is checked only when E0.f typed the field: a field without a
//! type already has its own diagnostic, and one that does not exist is `E0C-61` in E0.e.

use std::collections::HashMap;

use semantic_compiler_contracts::types::common::LogicalType;

use super::output::{ElementKind, Typing};
use super::value_type::{Typed, describe};
use crate::extensions::{ExtensionCode, ExtensionDiagnostic};
use crate::names::Name;
use crate::outline::{DatasetOutline, outline};
use crate::pointer::JsonPointer;
use crate::references::context::{ContextMetric, ContextMetrics};
use crate::structure::Structure;

/// Checks the kind of the time dimension of every context metric of `context`, in the
/// semantic model that `structure` selected, with the field types of `typing`.
///
/// # Errors
///
/// Returns one `E0C-61` diagnostic per context metric whose time dimension E0.f typed with a
/// kind other than `date` or `timestamp`, at the entry's `data` member (E0C-2.6), in document
/// order. `E0C-61` rejects the model (E0-4).
pub fn check_context_time_kinds(
    structure: &Structure<'_>,
    typing: &Typing,
    context: &ContextMetrics,
) -> Result<(), Vec<ExtensionDiagnostic>> {
    let model = outline(
        structure.semantic_model.node,
        &structure.semantic_model.pointer,
    );
    let datasets: HashMap<&Name, &DatasetOutline<'_>> = model
        .datasets
        .iter()
        .filter_map(|dataset| Some((&dataset.name.as_ref()?.name, dataset)))
        .collect();
    let field_types: HashMap<&JsonPointer, &Typed> = typing
        .elements
        .iter()
        .filter(|element| element.kind == ElementKind::Field)
        .map(|element| (&element.pointer, &element.typed))
        .collect();
    let errors: Vec<ExtensionDiagnostic> = context
        .metrics
        .iter()
        .filter_map(|metric| {
            let time = &metric.modifier.time()?.time_dimension;
            let field = datasets.get(&time.dataset)?.field(&time.field)?;
            let typed = field_types.get(&field.pointer)?;
            let is_time = matches!(
                typed.logical(),
                Some(LogicalType::Date | LogicalType::Timestamp { .. })
            );
            (!is_time).then(|| time_kind_diagnostic(metric, &time.written, typed))
        })
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// The `E0C-61` diagnostic of `metric`, whose time dimension `written` has the type `typed`.
fn time_kind_diagnostic(
    metric: &ContextMetric,
    written: &str,
    typed: &Typed,
) -> ExtensionDiagnostic {
    ExtensionDiagnostic {
        code: ExtensionCode::ContextTimeDimension,
        pointer: metric.diagnostic_pointer.clone(),
        location: metric.diagnostic_location,
        message: format!(
            "the `time_dimension` `{written}` of the context metric `{}` is {}, not a `date` or `timestamp` field",
            metric.name,
            describe(&typed.value_type)
        ),
    }
}
