//! The scenarios whose instructions Callgrind counts (R7): each reference query and each
//! point of the R3 sweep, on the reference model.

use crate::error::BenchmarkError;
use crate::inputs::{ModelInputs, compile_query};
use crate::model_shape::ModelShape;
use crate::queries::{
    BenchmarkQuery, SWEEP_ITEMS, reference_queries, security_context_document, sweep_query,
};

/// Every scenario query: the reference queries, then the sweep queries in item order.
#[must_use]
pub fn scenario_queries() -> Vec<BenchmarkQuery> {
    let shape = ModelShape::reference();
    let mut all = reference_queries(&shape);
    all.extend(SWEEP_ITEMS.iter().map(|items| sweep_query(&shape, *items)));
    all
}

/// Generates every input, loads the reference model and compiles the scenario `name`
/// `compilations` times. The instructions of one compilation are the difference between two
/// runs that differ only in `compilations`.
///
/// # Errors
///
/// [`BenchmarkError::UnknownScenario`] for a name of no scenario, and the errors of loading
/// and compiling.
pub fn run_scenario(name: &str, compilations: usize) -> Result<(), BenchmarkError> {
    let queries = scenario_queries();
    let query = queries
        .iter()
        .find(|query| query.name == name)
        .ok_or_else(|| BenchmarkError::UnknownScenario(name.to_owned()))?;
    let handle = ModelInputs::of(&ModelShape::reference()).load()?;
    let context = security_context_document();
    for _ in 0..compilations {
        compile_query(&handle, query, &context)?;
    }
    Ok(())
}
