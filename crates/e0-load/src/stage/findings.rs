//! The diagnostics of one substage, each with the code the load reports and whether it rejects
//! the model (E0-4.1), before they are ordered (E0-4.3).

use crate::classification::{ClassificationCode, ClassificationDiagnostic};
use crate::extensions::ExtensionDiagnostic;
use crate::pointer::{JsonPointer, position};
use crate::read::{Location, Node};
use crate::references::ReferenceDiagnostic;
use crate::relationships::RelationshipDiagnostic;
use crate::structure::StructureDiagnostic;
use crate::typing::{TYPING_WARNING, TypingDiagnostic};

/// One diagnostic of a substage, as the load reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Finding {
    /// The reported code: the rejecting code itself, or the load warning (E0-4 table).
    pub(super) code: &'static str,
    /// Whether the diagnostic rejects the model (disposition *reject*, E0-4.1).
    pub(super) rejects: bool,
    /// The JSON Pointer of the node it is about.
    pub(super) pointer: JsonPointer,
    /// Where that node starts in the document text.
    pub(super) location: Location,
    /// The English message.
    pub(super) message: String,
}

/// The findings of one substage (E0-3), in any order until [`Substage::ordered`].
#[derive(Debug, Default)]
pub(super) struct Substage {
    findings: Vec<Finding>,
}

impl Substage {
    /// Whether the substage fails: it reports a rejecting diagnostic (E0-3.1).
    pub(super) fn fails(&self) -> bool {
        self.findings.iter().any(|finding| finding.rejects)
    }

    /// Adds every finding of `findings`.
    pub(super) fn extend(&mut self, findings: impl IntoIterator<Item = Finding>) {
        self.findings.extend(findings);
    }

    /// The findings in document order, then by code (E0-4.3). The sort is stable, so findings
    /// at the same node with the same code keep the order the substage gave them.
    pub(super) fn ordered(mut self, document: &Node) -> Vec<Finding> {
        self.findings
            .sort_by_cached_key(|finding| (position(document, &finding.pointer), finding.code));
        self.findings
    }
}

/// A structure diagnostic (E0.b): `E0B-01` to `E0B-07` reject; `W0B-01` to `W0B-03` warn.
pub(super) fn structure(diagnostic: &StructureDiagnostic) -> Finding {
    Finding {
        code: diagnostic.code.as_str(),
        rejects: !diagnostic.code.is_warning(),
        pointer: diagnostic.pointer.clone(),
        location: diagnostic.location,
        message: diagnostic.message.clone(),
    }
}

/// An extension diagnostic (E0.c, and the extension checks of E0.d to E0.f): every code
/// rejects except the load warnings, and `E0C-37` is reported as `W0G-01`.
pub(super) fn extension(diagnostic: &ExtensionDiagnostic) -> Finding {
    Finding {
        code: diagnostic.code.load_code(),
        rejects: !diagnostic.code.is_warning(),
        pointer: diagnostic.pointer.clone(),
        location: diagnostic.location,
        message: diagnostic.message.clone(),
    }
}

/// A reference diagnostic (E0.d): `E0D-01`, `E0D-02` and `E0D-08` reject; the others are
/// reported as `W0D-01` or `W0F-01`.
pub(super) fn reference(diagnostic: &ReferenceDiagnostic) -> Finding {
    Finding {
        code: diagnostic.code.load_code(),
        rejects: diagnostic.code.rejects(),
        pointer: diagnostic.pointer.clone(),
        location: diagnostic.location,
        message: diagnostic.message.clone(),
    }
}

/// A relationship diagnostic (E0.e): never rejects; `E0E-01` and `E0E-02` are reported as
/// `W0E-04`.
pub(super) fn relationship(diagnostic: &RelationshipDiagnostic) -> Finding {
    Finding {
        code: diagnostic.code.load_code(),
        rejects: false,
        pointer: diagnostic.pointer.clone(),
        location: diagnostic.location,
        message: diagnostic.message.clone(),
    }
}

/// A typing diagnostic (E0.f): never rejects; reported as `W0F-01`.
pub(super) fn typing(diagnostic: &TypingDiagnostic) -> Finding {
    Finding {
        code: TYPING_WARNING,
        rejects: false,
        pointer: diagnostic.pointer.clone(),
        location: diagnostic.location,
        message: diagnostic.message.clone(),
    }
}

/// A classification diagnostic (E0.g): never rejects; reported as `W0G-01`.
pub(super) fn classification(diagnostic: &ClassificationDiagnostic) -> Finding {
    Finding {
        code: ClassificationCode::WARNING,
        rejects: false,
        pointer: diagnostic.pointer.clone(),
        location: diagnostic.location,
        message: diagnostic.message.clone(),
    }
}
