//! A diagnostic together with the element it makes unusable, as checks report it.

use super::diagnostic::{ReferenceCode, ReferenceDiagnostic};
use super::outline::LocatedText;
use super::output::ElementKind;
use crate::pointer::JsonPointer;
use crate::read::Location;

/// One problem found by a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    /// The diagnostic.
    pub(crate) diagnostic: ReferenceDiagnostic,
    /// The element made unusable, for a missing reference; `None` when the code rejects.
    pub(crate) element: Option<(ElementKind, JsonPointer)>,
}

impl Finding {
    /// A rejecting finding about the string `written`.
    pub(crate) fn rejecting(
        code: ReferenceCode,
        written: &LocatedText<'_>,
        message: String,
    ) -> Self {
        Self {
            diagnostic: ReferenceDiagnostic {
                code,
                pointer: written.pointer.clone(),
                location: written.location,
                message,
            },
            element: None,
        }
    }

    /// A missing reference at `pointer` that makes the element `kind` at `element` unusable.
    pub(crate) fn missing(
        code: ReferenceCode,
        (pointer, location): (&JsonPointer, Location),
        message: String,
        (kind, element): (ElementKind, &JsonPointer),
    ) -> Self {
        Self {
            diagnostic: ReferenceDiagnostic {
                code,
                pointer: pointer.clone(),
                location,
                message,
            },
            element: Some((kind, element.clone())),
        }
    }
}

impl LocatedText<'_> {
    /// Where the string is, as [`Finding::missing`] takes it.
    pub(crate) fn place(&self) -> (&JsonPointer, Location) {
        (&self.pointer, self.location)
    }
}
