//! The public functions `dependencies` and `diff` (architecture section 3.5): the model-level
//! dependency graph (DEP-1 to DEP-3) and the comparison of two of them (DEP-4).

mod diff;
mod elements;
mod hashes;
mod inputs;
mod metrics;
mod names;
mod resolve;
mod usability;
mod visibility;

pub use diff::diff;

use menura_contracts::ContractKind;
use menura_contracts::types::compiled_plan::{CompileErrorContract, CompileErrorDocument};
use menura_contracts::types::dependencies::{ModelDependenciesContract, ModelDependenciesDocument};

use crate::handle::Handle;
use crate::producer::producer;
use crate::stop::Stop;
use resolve::Resolver;

/// The model-dependency document of the model of `handle` (DEP-1): for each dataset,
/// relationship, field, metric and row policy, the model elements, catalog columns and row
/// policies it depends on, and its definition hash.
///
/// It is a pure function of the handle's model (DEP-3.1): it needs no security context, reads
/// no catalog state, and the same model gives the same document (DEP-3.2). It lists every
/// element, authorized or not, so it is for modelers and governance tools, never end users.
///
/// # Errors
///
/// A [`CompileErrorDocument`] with an internal error when the build has no valid producer.
pub fn dependencies(handle: &Handle) -> Result<ModelDependenciesDocument, CompileErrorDocument> {
    let producer = producer().map_err(|error| CompileErrorDocument {
        contract: CompileErrorContract::CompileError,
        contract_version: ContractKind::CompileError.latest_version(),
        errors: vec![Stop::from(error).into_diagnostic()],
        warnings: handle.warnings().cloned().collect(),
    })?;
    let resolver = Resolver::new(handle.model(), handle.catalog_schema());
    Ok(ModelDependenciesDocument {
        contract: ModelDependenciesContract::ModelDependencies,
        contract_version: ContractKind::ModelDependencies.latest_version(),
        producer,
        model_hash: handle.model().hash().clone(),
        datasets: elements::datasets(&resolver),
        relationships: elements::relationships(&resolver),
        fields: elements::fields(&resolver),
        metrics: elements::metrics(&resolver),
        policies: elements::policies(&resolver),
    })
}
