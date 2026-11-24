//! The source check of E6-7.1: `ClickHouse` names a table with at most two parts, `table` or
//! `database.table`, so a plan that scans a relation `source` of three or more parts stops
//! with `E6-01` before the check of E6-7.12 and before any SQL is written.

use menura_model::internal_model::InternalModel;
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::origin::Origin;

use super::post_order;
use crate::error::GenerateError;
use crate::identifiers::{Source, classify_source};

/// The most parts a relation `source` may have: `database.table` (E6-7.1).
const MAXIMUM_PARTS: usize = 2;

/// `E6-01` at the first dataset, among those `plan` scans in the order its steps are written
/// (E6-4.12), whose relation `source` has three or more parts. A query `source`, and a
/// `source` that is neither a query nor a qualified name, are left to the steps.
///
/// # Errors
///
/// [`GenerateError::InexpressibleSource`] when such a dataset exists.
pub(crate) fn check_sources(plan: &PlanNode, model: &InternalModel) -> Result<(), GenerateError> {
    let mut nodes = Vec::new();
    post_order(plan, &mut nodes);
    let first = nodes
        .iter()
        .filter_map(|node| match node.operation() {
            Operation::Scan { dataset, .. } => model.dataset(dataset),
            _ => None,
        })
        .find(|dataset| {
            matches!(
                classify_source(&dataset.source),
                Some(Source::Relation(parts)) if parts.len() > MAXIMUM_PARTS
            )
        });
    match first {
        Some(dataset) => Err(GenerateError::InexpressibleSource {
            origin: Origin::Model(dataset.location.clone()),
            dataset: dataset.name.as_str().to_owned(),
        }),
        None => Ok(()),
    }
}
