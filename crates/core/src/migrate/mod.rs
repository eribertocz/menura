//! The public function `migrate` (architecture section 3.5, MIG-1): rewrites the deprecated
//! forms of a model document into their current forms and reports each rewrite.

mod data_text;
mod entries;
mod location;
mod namespace;

use menura_contracts::types::compiled_plan::{CompileErrorContract, CompileErrorDocument};
use menura_contracts::types::migration_report::{MigrationReportContract, MigrationReportDocument};
use menura_contracts::{ContractKind, FORMER_NAMESPACE};
use menura_e0_load::{LoadRejection, read_model_tree};

use crate::load_options::LoadOptions;
use crate::producer::producer;
use crate::stop::Stop;
use entries::extension_entries;
use namespace::rewrite_former_namespace;

/// Migrates the model document `model`, read with the load options `options`, and returns the
/// migration report (MIG-2): the model with every deprecated form rewritten (MIG-3.2), and one
/// change per rewrite, in document order (MIG-3.4).
///
/// The model is read by E0.a and E0.b only, and every model of the document is covered, so
/// `model_name` is ignored (MIG-3.1). Nothing but the rewrites changes: member order, every
/// model and the format are kept (MIG-3.3). Migrating the model of a report gives no change
/// (MIG-3.5), and the same document and options give the same report (MIG-3.6).
///
/// # Errors
///
/// The compile error of E0.a or E0.b, without warnings, when either rejects the document
/// (MIG-3.1); an internal error when the build has no valid producer.
///
/// # Example
///
/// ```
/// use menura_core::{LoadOptions, migrate};
///
/// let model = br#"{"version": "0.1.1", "semantic_model": [{"name": "m",
///     "datasets": [{"name": "d", "source": "s.t", "custom_extensions": [{"vendor_name": "COMMON",
///     "data": "{\"namespace\": \"semantic_compiler\", \"version\": 1}"}]}]}]}"#;
/// let report = migrate(model, &LoadOptions::default()).unwrap();
/// assert_eq!(report.changes[0].pointer, "/semantic_model/0/datasets/0/custom_extensions/0/data");
/// assert_eq!(report.changes[0].to, "menura");
/// ```
pub fn migrate(
    model: &[u8],
    options: &LoadOptions,
) -> Result<MigrationReportDocument, CompileErrorDocument> {
    let mut tree = read_model_tree(model, options.ossie_format).map_err(
        |LoadRejection { errors, warnings }| CompileErrorDocument {
            contract: CompileErrorContract::CompileError,
            contract_version: ContractKind::CompileError.latest_version(),
            errors,
            warnings,
        },
    )?;
    let producer = producer().map_err(|error| CompileErrorDocument {
        contract: CompileErrorContract::CompileError,
        contract_version: ContractKind::CompileError.latest_version(),
        errors: vec![Stop::from(error).into_diagnostic()],
        warnings: Vec::new(),
    })?;
    let mut changes = Vec::new();
    if options.namespace != FORMER_NAMESPACE {
        for (entry, location) in extension_entries(&mut tree, options.ossie_format) {
            changes.extend(rewrite_former_namespace(
                entry,
                &location,
                options.ossie_format,
                &options.namespace,
            ));
        }
    }
    changes.sort_by(|(earlier, _), (later, _)| earlier.cmp(later));
    Ok(MigrationReportDocument {
        contract: MigrationReportContract::MigrationReport,
        contract_version: ContractKind::MigrationReport.latest_version(),
        producer,
        changes: changes.into_iter().map(|(_, change)| change).collect(),
        model: tree,
    })
}
