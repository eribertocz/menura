//! Stage E4: builds the logical plan (E4-1, E4-2).
//!
//! In the 0.4.0 track [`build_plan`] builds the plan of each branch: `Scan` of the home
//! dataset, which reads its visible rows (the row-policy `Filter`, then one `SemiJoin` per
//! relationship that propagation follows, E4-6.5), one left `Join` per join instance, whose
//! `Scan` reads the visible rows of its dataset the same way (E4-6.5c), the query-filter
//! `Filter`, `Deduplicate` when the branch joins and has metric components, and `Aggregate`
//! (E4-4). Two or more branches are combined by a `Stitch` (E4-5). Then come `Project`,
//! `Sort` and `Limit` (E4-9). A derived metric is expanded into the components of its inputs,
//! and its formula is computed in the final `Project` over them (DER-4.1). Additivity and
//! context modifiers come in later issues; an input that needs them is an internal error.
//!
//! The plan follows the conventions shared with E6 (issue #251, E6-4.12): the home `Scan`
//! projects fields of its dataset by normalized name, and the source columns a dataset
//! metric reads under a prefix that no field name of the dataset starts with (issue #264);
//! the `Scan` of a join instance projects its fields under the prefix of the instance; every
//! other node reads the columns of its input by unqualified reference; the query-filter
//! `Filter` comes from the query, and the row-policy `Filter`, the `Join`, `Deduplicate` and
//! `Aggregate` nodes and the final `Project` come from the model, so their literals are model
//! constants.

#![forbid(unsafe_code)]

mod columns;
mod derived_inputs;
mod dimensions;
mod error;
mod expansion;
mod followed;
mod joins;
mod layout;
mod metrics;
mod nodes;
mod output;
mod policy;
mod policy_visits;
mod population;
mod query_filter;
mod rewrite;
mod rows;
mod scan;
mod slice;
#[expect(
    dead_code,
    reason = "used by the semi-additive branch plan, which follows in this pull request"
)]
mod snapshot_grain;
mod stitch;
mod typing;
mod visibility;

pub use error::{InvariantViolation, PlanError};
pub use population::{PopulationRequest, plan_population};

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::PlanNode;
use menura_representations::origin::Origin;
use menura_representations::paths::{Branch, Branches};
use menura_security::AdmittedSecurityContext;

use crate::layout::Layout;
use crate::nodes::aggregate;

/// Builds the logical plan of `query` from its `branches`, for the user of `security`, over
/// `model` and the admitted `catalog` schema, which gives the type of every source column a
/// dataset metric reads (E4-2, CAT-4).
///
/// # Errors
///
/// [`PlanError::MissingAttribute`] (`ESEC-02`, E2-5 row 18) when an applicable row policy
/// needs an attribute `security` does not provide; then [`PlanError::MissingReferenceTime`]
/// (`EQRY-03`, E4-6.9b) when an applicable row policy reads the current date or time and the
/// query has no `reference_time`; [`PlanError::Invariant`] for an input that the stages
/// before E4 never produce, or that needs a later part of E4.
pub fn build_plan(
    branches: &Branches,
    query: &BoundQuery,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, PlanError> {
    let first = slice::first_branch(branches)?;
    let metrics = metrics::planned_metrics(query, model)?;
    let outputs = metrics::metric_outputs(branches, &metrics)?;
    let home_origin = Origin::Model(slice::home_dataset(first, model)?.location.clone());
    let project_origin =
        metrics::first_metric_location(&metrics).map_or(home_origin, Origin::Model);
    let mut plans = branches
        .as_slice()
        .iter()
        .map(|branch| branch_plan(branch, query, security, model, catalog, &metrics))
        .collect::<Result<Vec<_>, _>>()?;
    let combined = match plans.len() {
        0 => return Err(slice::no_branch()),
        1 => plans.pop().ok_or_else(slice::no_branch)?,
        _ => stitch::stitch(plans, project_origin.clone())?,
    };
    let with_inputs = derived_inputs::compute_inputs(
        combined,
        &metrics::input_columns(&metrics),
        &project_origin,
    )?;
    let projected = output::project(query, with_inputs, outputs, project_origin)?;
    output::order_and_limit(query, projected)
}

/// The plan of `branch`, up to its `Aggregate` (E4-4).
fn branch_plan(
    branch: &Branch,
    query: &BoundQuery,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
    metrics: &[metrics::PlannedMetric<'_>],
) -> Result<PlanNode, PlanError> {
    let home = slice::home_dataset(branch, model)?;
    let layout = Layout::new(branch, home, model)?;
    let keys = dimensions::dimension_keys(query, &layout)?;
    let components = metrics::branch_components(branch, metrics, home)?;
    let source = rows::BranchSource {
        branch,
        home,
        layout: &layout,
        security,
        model,
        catalog,
        has_reference_time: query.time().reference_time.is_some(),
    };
    let request = rows::RowsRequest {
        keys: &keys,
        components: &components,
        fields: &[],
        predicate: query_filter::query_predicate(query, &layout)?,
    };
    let node = rows::branch_rows(&source, request)?;
    aggregate(
        node,
        keys,
        components,
        &Origin::Model(home.location.clone()),
    )
}
