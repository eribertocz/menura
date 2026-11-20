//! Every load warning of E0, in the order of E0-4.3: substage order, then document order, then
//! by code. E0.h reports none of its own.

use menura_model::internal_model::{DiagnosticCode, Warning};

use super::ModelInput;
use super::located::located;
use crate::classification::ClassificationCode;
use crate::pointer::{JsonPointer, position};
use crate::read::{Location, Node};
use crate::typing::TYPING_WARNING;

/// One warning before it is ordered.
struct Found<'input> {
    code: &'static str,
    pointer: &'input JsonPointer,
    location: Location,
    message: &'input str,
}

/// Every diagnostic of `$warnings` as a [`Found`], with the load code `$code` gives.
macro_rules! found {
    ($warnings:expr, |$warning:ident| $code:expr) => {
        $warnings.map(|$warning| Found {
            code: $code,
            pointer: &$warning.pointer,
            location: $warning.location,
            message: &$warning.message,
        })
    };
}

/// The warnings of every substage of `input`, in report order.
pub(super) fn warnings(document: &Node, input: &ModelInput<'_, '_>) -> Vec<Warning> {
    let extension_warnings = input
        .extensions
        .warnings
        .iter()
        .chain(&input.access.warnings);
    let typing_warnings = input
        .typing
        .warnings
        .iter()
        .chain(&input.derived_typing.warnings);
    let substages: [Vec<Found<'_>>; 6] = [
        found!(input.structure.warnings.iter(), |warning| warning
            .code
            .as_str())
        .collect(),
        found!(extension_warnings, |warning| warning.code.load_code()).collect(),
        found!(input.references.warnings.iter(), |warning| warning
            .code
            .load_code())
        .chain(found!(input.derived.warnings.iter(), |warning| warning
            .code
            .load_code()))
        .collect(),
        found!(input.relationships.warnings.iter(), |warning| warning
            .code
            .load_code())
        .collect(),
        found!(typing_warnings, |warning| TYPING_WARNING).collect(),
        found!(input.classification.warnings.iter(), |warning| {
            ClassificationCode::WARNING
        })
        .collect(),
    ];
    let mut ordered = Vec::new();
    for substage in substages {
        ordered.extend(in_document_order(document, substage));
    }
    ordered
}

/// The warnings of one substage, in document order then by code; the sort is stable, so the
/// order a substage gave to warnings at the same place and with the same code is kept.
fn in_document_order(document: &Node, mut found: Vec<Found<'_>>) -> Vec<Warning> {
    found.sort_by_cached_key(|warning| (position(document, warning.pointer), warning.code));
    found
        .into_iter()
        .map(|warning| Warning {
            code: DiagnosticCode::new(warning.code),
            message: warning.message.to_owned(),
            location: located(warning.pointer, warning.location),
        })
        .collect()
}
