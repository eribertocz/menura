//! The diagnostics of a load in report order (E0-3.2, E0-4.3): the catalog schema errors
//! first, then each substage in order, each in document order then by code.

use menura_contracts::AdmissionError;
use menura_contracts::types::compiled_plan::{Diagnostic, DiagnosticLocation, InputDocument};
use menura_model::internal_model::{DiagnosticCode, InternalModel, SourceLocation, Warning};

use super::LoadRejection;
use super::findings::{Finding, Substage};
use crate::read::{Node, ReadError};

/// Every diagnostic reported so far, in report order.
#[derive(Debug, Default)]
pub(super) struct Report {
    /// The rejecting diagnostics.
    errors: Vec<Diagnostic>,
    /// The load warnings, as the compile error document reports them.
    warnings: Vec<Diagnostic>,
    /// The same load warnings, as the internal model keeps them (E0H-2).
    model_warnings: Vec<Warning>,
}

impl Report {
    /// Whether an error has been reported: the model does not load.
    pub(super) fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Adds the error of a catalog schema that is not admitted (CAT-9.1).
    pub(super) fn add_catalog_error(&mut self, error: &AdmissionError) {
        let (line, column) = error
            .line_column
            .map_or((None, None), |(line, column)| (Some(line), Some(column)));
        self.add_error(Diagnostic {
            code: error.code.as_str().to_owned(),
            message: error.message.clone(),
            location: Some(DiagnosticLocation {
                document: error.document,
                pointer: error.pointer.clone(),
                line,
                column,
            }),
        });
    }

    /// Adds an error that is already a diagnostic of the compile error document.
    pub(super) fn add_error(&mut self, diagnostic: Diagnostic) {
        self.errors.push(diagnostic);
    }

    /// Adds the error of a model document that cannot be read (E0.a). The data tree does not
    /// exist, so the error is located by line and column at the empty pointer.
    pub(super) fn add_read_error(&mut self, error: &ReadError) {
        self.errors.push(Diagnostic {
            code: error.code.as_str().to_owned(),
            message: error.message.clone(),
            location: Some(model_location(
                "",
                error.location.line,
                error.location.column,
            )),
        });
    }

    /// Adds the findings of one substage, in document order then by code, and returns whether
    /// the substage failed (E0-3.1).
    pub(super) fn add_substage(&mut self, document: &Node, substage: Substage) -> bool {
        let failed = substage.fails();
        for finding in substage.ordered(document) {
            self.add_finding(finding);
        }
        failed
    }

    fn add_finding(&mut self, finding: Finding) {
        let Finding {
            code,
            rejects,
            pointer,
            location,
            message,
        } = finding;
        let diagnostic = Diagnostic {
            code: code.to_owned(),
            message: message.clone(),
            location: Some(model_location(
                pointer.as_str(),
                location.line,
                location.column,
            )),
        };
        if rejects {
            self.errors.push(diagnostic);
            return;
        }
        self.warnings.push(diagnostic);
        self.model_warnings.push(Warning {
            code: DiagnosticCode::new(code),
            message,
            location: SourceLocation {
                pointer: pointer.as_str().to_owned(),
                line: location.line,
                column: location.column,
            },
        });
    }

    /// The internal model with every load warning of the report (E0H-2). The warnings are not
    /// part of any hash (E0H-3.4), so the model hash is kept.
    pub(super) fn seal(self, model: &InternalModel) -> InternalModel {
        let hash = model.hash().clone();
        let mut parts = model.parts().clone();
        parts.warnings = self.model_warnings;
        InternalModel::new(parts, hash)
    }

    /// The rejection: every error and every warning reported.
    pub(super) fn into_rejection(self) -> LoadRejection {
        LoadRejection {
            errors: self.errors,
            warnings: self.warnings,
        }
    }
}

/// A location in the model document.
fn model_location(pointer: &str, line: usize, column: usize) -> DiagnosticLocation {
    DiagnosticLocation {
        document: InputDocument::Model,
        pointer: pointer.to_owned(),
        line: u64::try_from(line).ok(),
        column: u64::try_from(column).ok(),
    }
}
