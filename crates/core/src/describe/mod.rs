//! The public function `describe` (architecture section 3.5): what one user may ask of one
//! model (DSC-1).

mod ai_context;
mod capabilities;
mod datasets;
mod graph;
mod listing;
mod metrics;

use menura_contracts::ContractKind;
use menura_contracts::types::compiled_plan::{
    CompileErrorContract, CompileErrorDocument, Diagnostic,
};
use menura_contracts::types::descriptions::{ModelDescriptionContract, ModelDescriptionDocument};
use menura_security::Authorization;

use crate::compile::admit_context;
use crate::handle::Handle;
use crate::stop::Stop;
use graph::DescribedGraph;
use listing::Listing;

/// What `describe` returns: the model description, or the compile error of a security context
/// that is not admitted (DSC-3.12).
#[derive(Debug, Clone, PartialEq)]
pub enum DescribeResult {
    /// The model description (DSC-2).
    Description(Box<ModelDescriptionDocument>),
    /// The compile error of the security context (SEC-3.8).
    Error(CompileErrorDocument),
}

/// Describes the model of `handle` for the user of the security context document
/// `security_context` (DSC-1): the datasets, fields and metrics the user may ask for, the
/// pairs of datasets with more than one path, and the capability profile of the release.
///
/// It is a pure function of the model and the context (DSC-3.5): the same inputs give the
/// same document, with every list ordered by normalized name (DSC-3.4). The context is
/// admitted exactly as `compile` admits it (DSC-3.12); a context read through the former
/// contract name gives the same description, and a description carries no warnings.
///
/// # Errors
///
/// [`DescribeResult::Error`], the compile error document with the handle's load warnings and
/// the error SEC-3.8 gives for a context that is not admitted (`ESEC-01` or `ESEC-03`), or
/// `E_INTERNAL` for a defect of the compiler (PLN-5.1a).
///
/// # Example
///
/// ```
/// use menura_core::{DescribeResult, LoadOptions, describe, load};
/// # let model = include_bytes!("../../tests/fixtures/shop/model.yaml");
/// # let catalog_schema = include_bytes!("../../tests/fixtures/shop/catalog-schema.json");
///
/// let handle = load(model, catalog_schema, None, &LoadOptions::default()).unwrap();
/// let context = br#"{"contract": "menura.security_context", "contract_version": "1.1.0",
///     "subject": "user-1", "roles": ["admin"], "attributes": {}}"#;
/// let DescribeResult::Description(description) = describe(&handle, context) else {
///     panic!("the context is admitted");
/// };
/// assert_eq!(description.datasets[0].name, "CUSTOMERS");
/// ```
#[must_use]
pub fn describe(handle: &Handle, security_context: &[u8]) -> DescribeResult {
    match description(handle, security_context) {
        Ok(document) => DescribeResult::Description(Box::new(document)),
        Err(stop) => DescribeResult::Error(CompileErrorDocument {
            contract: CompileErrorContract::CompileError,
            contract_version: ContractKind::CompileError.latest_version(),
            errors: vec![stop.into_diagnostic()],
            warnings: handle.warnings().cloned().collect::<Vec<Diagnostic>>(),
        }),
    }
}

/// The description of the model of `handle` for the user of `security_context`.
fn description(handle: &Handle, security_context: &[u8]) -> Result<ModelDescriptionDocument, Stop> {
    let context = admit_context(handle, security_context)?;
    let model = handle.model();
    let authorization = Authorization::new(model, &context);
    let listing = Listing::new(model, &authorization);
    let graph = DescribedGraph::new(model, &listing, &authorization);
    Ok(ModelDescriptionDocument {
        contract: ModelDescriptionContract::ModelDescription,
        contract_version: ContractKind::ModelDescription.latest_version(),
        model_hash: model.hash().clone(),
        security_fingerprint: context.fingerprint(),
        datasets: datasets::described_datasets(&listing, model.hash()),
        metrics: metrics::described_metrics(&listing, &graph, model.hash()),
        ambiguous_paths: Some(graph.ambiguous_paths()),
        capabilities: Some(capabilities::capabilities()?),
    })
}
