//! Substages E0.e, E0.f and E0.g of the stage function: relationships, types and metric
//! classification. Each runs the checks of the model's own elements, and the extension checks
//! only when E0.c and the substage they depend on succeeded (E0-3.3, E0-4 extension table).

use semantic_compiler_contracts::types::catalog::CatalogSchemaDocument;

use super::findings::{self, Substage};
use super::parsing::parse_warnings;
use super::report::Report;
use super::resolution::{Resolved, add_errors};
use crate::classification::{Classification, ClassificationInput, classify_metrics};
use crate::extensions::Extensions;
use crate::extensions::summaries::SummaryDatasets;
use crate::read::Node;
use crate::relationships::additivity::check_additivity_reachability;
use crate::relationships::context::{ResolvedMetrics, check_context_reachability};
use crate::relationships::{Relationships, analyze_relationships};
use crate::structure::Structure;
use crate::typing::additivity::type_additivity;
use crate::typing::context::check_context_time_kinds;
use crate::typing::derived::type_derived_metrics;
use crate::typing::populations::type_populations;
use crate::typing::security::type_row_policies;
use crate::typing::summaries::type_summaries;
use crate::typing::{Typing, type_expressions};

/// What the substages after E0.d read.
pub(super) struct Earlier<'run, 'document> {
    /// The model document.
    pub(super) document: &'run Node,
    /// What E0.b found.
    pub(super) structure: &'run Structure<'document>,
    /// The admitted catalog schema.
    pub(super) catalog: &'run CatalogSchemaDocument,
    /// The summary datasets (SUM-6.1.7).
    pub(super) summaries: &'run SummaryDatasets,
    /// What E0.c read, or only the summaries when E0.c failed.
    pub(super) extensions: &'run Extensions,
    /// Whether E0.c succeeded, so the extension checks run (E0-3.3).
    pub(super) extension_checks: bool,
    /// What E0.d resolved.
    pub(super) resolved: &'run Resolved,
}

/// The result of E0.e: the graph, and whether the substage failed.
pub(super) struct Analyzed {
    /// The relationship graph.
    pub(super) relationships: Relationships,
    /// Whether an extension check of E0.e rejected the model.
    pub(super) failed: bool,
}

/// Runs E0.e and adds its diagnostics to `report`.
pub(super) fn relationships(earlier: &Earlier<'_, '_>, report: &mut Report) -> Analyzed {
    let Earlier {
        document,
        structure,
        extensions,
        resolved,
        ..
    } = earlier;
    let references = &resolved.references;
    let relationships =
        analyze_relationships(document, &structure.semantic_model, extensions, references);
    let mut substage = Substage::default();
    substage.extend(relationships.warnings.iter().map(findings::relationship));
    if earlier.extension_checks {
        let additivity =
            check_additivity_reachability(structure, extensions, references, &relationships);
        add_errors(&mut substage, additivity);
        let metrics = ResolvedMetrics {
            derived: &resolved.derived,
            context: &resolved.context,
        };
        let context = check_context_reachability(structure, references, metrics, &relationships);
        add_errors(&mut substage, context);
    }
    let failed = report.add_substage(document, substage);
    Analyzed {
        relationships,
        failed,
    }
}

/// The types E0.f found.
pub(super) struct Typed {
    /// The types of fields and the metrics that are neither derived nor context metrics.
    pub(super) typing: Typing,
    /// The types of derived and context metrics; empty when the extension checks did not run.
    pub(super) derived_typing: Typing,
}

/// Runs E0.f and adds its diagnostics to `report`; `None` when E0.f fails. The extension checks
/// run only when `extension_checks` holds and E0.e succeeded (`relationships_failed`).
pub(super) fn types(
    earlier: &Earlier<'_, '_>,
    relationships_failed: bool,
    report: &mut Report,
) -> Option<Typed> {
    let Earlier {
        document,
        structure,
        catalog,
        summaries,
        resolved,
        ..
    } = earlier;
    let typing = type_expressions(document, structure, catalog, &resolved.derived, summaries);
    let mut substage = Substage::default();
    substage.extend(typing.warnings.iter().map(findings::typing));
    substage.extend(parse_warnings(document, structure));
    let derived_typing = if earlier.extension_checks && !relationships_failed {
        extension_types(earlier, &typing, &mut substage)
    } else {
        Some(Typing::default())
    };
    if report.add_substage(document, substage) {
        return None;
    }
    Some(Typed {
        typing,
        derived_typing: derived_typing?,
    })
}

/// Runs the extension checks of E0.f, adding their diagnostics to `substage`, and returns the
/// types of derived and context metrics.
fn extension_types(
    earlier: &Earlier<'_, '_>,
    typing: &Typing,
    substage: &mut Substage,
) -> Option<Typing> {
    let Earlier {
        document,
        structure,
        catalog,
        extensions,
        resolved,
        ..
    } = earlier;
    add_errors(substage, type_populations(structure, typing, extensions));
    add_errors(substage, type_additivity(structure, typing, extensions));
    let (derived, context) = (&resolved.derived, &resolved.context);
    let derived_typing =
        match type_derived_metrics(document, structure, catalog, typing, derived, context) {
            Ok(derived_typing) => {
                substage.extend(derived_typing.warnings.iter().map(findings::typing));
                Some(derived_typing)
            }
            Err(errors) => {
                substage.extend(errors.iter().map(findings::extension));
                None
            }
        };
    add_errors(
        substage,
        check_context_time_kinds(structure, typing, context),
    );
    add_errors(substage, type_row_policies(structure, typing, extensions));
    add_errors(substage, type_summaries(structure, typing, extensions));
    derived_typing
}

/// Runs E0.g and adds its diagnostics to `report`; it never rejects the model.
pub(super) fn classification(
    earlier: &Earlier<'_, '_>,
    typed: &Typed,
    report: &mut Report,
) -> Classification {
    let input = ClassificationInput {
        extensions: earlier.extensions,
        references: &earlier.resolved.references,
        derived: &earlier.resolved.derived,
        context: &earlier.resolved.context,
        typing: &typed.typing,
        derived_typing: &typed.derived_typing,
    };
    let classification = classify_metrics(earlier.document, earlier.structure, &input);
    let mut substage = Substage::default();
    substage.extend(classification.warnings.iter().map(findings::classification));
    report.add_substage(earlier.document, substage);
    classification
}
