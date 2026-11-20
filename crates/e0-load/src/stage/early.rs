//! Substages E0.b and E0.c of the stage function: the structure of the document and the
//! common extension rules, which run without the catalog schema (E0-3.2).

use serde_json::{Map, Value as Json};

use menura_contracts::types::audit_record::LoadOptions;

use super::findings::{self, Substage};
use super::report::Report;
use crate::extensions::security::Security;
use crate::extensions::security::resolution::{ResolvedAccess, resolve_access};
use crate::extensions::summaries::{AGGREGATE, SummaryDatasets};
use crate::extensions::{Extensions, ObjectKind, OwnEntry, validate_extensions};
use crate::pointer::resolve;
use crate::read::Node;
use crate::structure::{Structure, validate_structure};

/// The extensions of a model whose E0.c succeeded, and the access of every dataset.
pub(super) struct AcceptedExtensions {
    /// What E0.c read.
    pub(super) extensions: Extensions,
    /// The access every dataset is read with (SCY-5.1.7).
    pub(super) access: ResolvedAccess,
}

/// Runs E0.b and adds its diagnostics to `report`; `None` when the structure is rejected.
pub(super) fn structure<'document>(
    document: &'document Node,
    options: &LoadOptions,
    report: &mut Report,
) -> Option<Structure<'document>> {
    let mut substage = Substage::default();
    let format = options.ossie_format;
    let structure = match validate_structure(document, format, options.model_name.as_deref()) {
        Ok(structure) => {
            substage.extend(structure.warnings.iter().map(findings::structure));
            Some(structure)
        }
        Err(errors) => {
            substage.extend(errors.iter().map(findings::structure));
            None
        }
    };
    report.add_substage(document, substage);
    structure
}

/// Runs E0.c, with the access of every dataset and its warnings (`W0C-40`, `W0C-41`), and adds
/// its diagnostics to `report`; `None` when E0.c fails.
pub(super) fn extensions(
    document: &Node,
    structure: &Structure<'_>,
    summaries: &SummaryDatasets,
    options: &LoadOptions,
    report: &mut Report,
) -> Option<AcceptedExtensions> {
    let model = &structure.semantic_model;
    let mut substage = Substage::default();
    let accepted = match validate_extensions(document, model, &options.namespace) {
        Ok(extensions) => {
            let grant = options.grant_authenticated_to_all;
            let access = resolve_access(document, model, &extensions.security, summaries, grant);
            substage.extend(extensions.warnings.iter().map(findings::extension));
            substage.extend(access.warnings.iter().map(findings::extension));
            Some(AcceptedExtensions { extensions, access })
        }
        Err(failure) => {
            substage.extend(failure.errors.iter().map(findings::extension));
            substage.extend(failure.warnings.iter().map(findings::extension));
            None
        }
    };
    report.add_substage(document, substage);
    accepted
}

/// The extensions the own checks of E0.d to E0.g read when E0.c failed (E0-3.3): no
/// declaration of any extension, and only which datasets are summaries, which never depends on
/// what E0.c reported (SUM-6.1.7).
pub(super) fn summaries_only(document: &Node, summaries: &SummaryDatasets) -> Extensions {
    let entries = summaries
        .iter()
        .map(|pointer| OwnEntry {
            object: ObjectKind::Dataset,
            object_pointer: pointer.clone(),
            data_pointer: pointer.clone(),
            location: resolve(document, pointer).map_or(document.location, |node| node.location),
            extensions: Map::from_iter([(AGGREGATE.to_owned(), Json::Null)]),
        })
        .collect();
    Extensions {
        entries,
        populations: Vec::new(),
        additivity: Vec::new(),
        derived: Vec::new(),
        context: Vec::new(),
        security: Security::default(),
        summaries: Vec::new(),
        warnings: Vec::new(),
    }
}
