//! Stage E4: builds the logical plan (E4-1, E4-2).
//!
//! In the 0.4.0 track a query has one branch. [`build_plan`] builds its plan: `Scan` of the
//! home dataset, which reads its visible rows (the row-policy `Filter`, then one `SemiJoin`
//! per relationship that propagation follows, E4-6.5), one left `Join` per join instance,
//! whose `Scan` reads the visible rows of its dataset the same way (E4-6.5c), the
//! query-filter `Filter`, `Deduplicate` when the branch joins and has metric components,
//! `Aggregate`, `Project`, then `Sort` and `Limit` (E4-4, E4-9). `Stitch`, additivity and
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
mod dimensions;
mod error;
mod followed;
mod joins;
mod layout;
mod metrics;
mod nodes;
mod output;
mod policy;
mod policy_visits;
mod query_filter;
mod rewrite;
mod scan;
mod slice;
mod typing;
mod visibility;

pub use error::{InvariantViolation, PlanError};

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::expression::Expression;
use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{AggregateComponent, NamedExpression, PlanNode};
use menura_representations::origin::Origin;
use menura_representations::paths::Branches;
use menura_security::AdmittedSecurityContext;

use crate::columns::read_fields;
use crate::layout::Layout;
use crate::nodes::{aggregate, component_expression, deduplicate, filter};

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
    let branch = slice::single_branch(branches)?;
    let home = slice::home_dataset(branch, model)?;
    let layout = Layout::new(branch, home, model)?;
    let metrics = metrics::planned_metrics(query, model)?;
    let keys = dimensions::dimension_keys(query, &layout)?;
    let components = metrics::branch_components(branch, &metrics, home)?;
    let outputs = metrics::metric_outputs(branch, &metrics)?;
    let query_predicate = query_filter::query_predicate(query, &layout)?;
    let has_reference_time = query.time().reference_time.is_some();
    let row_filters = policy_visits::row_filters(home, model, security, has_reference_time)?;

    let mut read = fields_read(
        &keys,
        &components,
        query_predicate.as_ref().map(|(predicate, _)| predicate),
    );
    let visibility = visibility::Visibility {
        model,
        filters: &row_filters,
        catalog,
    };
    read.extend(joins::join_reads(&layout));
    read.extend(visibility.fields(home)?);
    for instance in layout.instances() {
        read.extend(visibility.instance_fields(instance.dataset, &instance.prefix)?);
    }
    let deduplicated = !branch.join_tree.is_empty() && !components.is_empty();
    let identity = deduplicated.then(|| layout.identity());
    let scan = scan::scan(home, &joins::home_reads(&layout, &read), catalog, identity)?;
    let mut node = visibility.visible_rows(scan, home, None)?;
    node = joins::join_instances(node, &layout, &read, &visibility)?;
    if let Some((predicate, location)) = query_predicate {
        node = filter(node, predicate, Origin::Query(location));
    }
    let home_origin = Origin::Model(home.location.clone());
    if let Some(identity) = identity {
        node = deduplicate(node, &keys, identity.clone(), home_origin.clone());
    }
    let aggregate = aggregate(node, keys, components, &home_origin)?;
    let project_origin =
        metrics::first_metric_location(&metrics).map_or(home_origin, Origin::Model);
    let projected = output::project(query, aggregate, outputs, project_origin)?;
    output::order_and_limit(query, projected)
}

/// The fields the keys, the components and the query predicate read.
fn fields_read(
    keys: &[NamedExpression],
    components: &[AggregateComponent],
    query_predicate: Option<&Expression>,
) -> Vec<NormalizedName> {
    let mut read = Vec::new();
    for key in keys {
        read_fields(&key.expression, &mut read);
    }
    for component in components {
        read_fields(&component_expression(component), &mut read);
    }
    if let Some(predicate) = query_predicate {
        read_fields(predicate, &mut read);
    }
    read
}
