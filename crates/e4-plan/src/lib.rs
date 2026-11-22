//! Stage E4: builds the logical plan (E4-1, E4-2).
//!
//! In the 0.4.0 track [`build_plan`] builds the plan of each branch: `Scan` of the home
//! dataset, which reads its visible rows (the row-policy `Filter`, then one `SemiJoin` per
//! relationship that propagation follows, E4-6.5), one left `Join` per join instance, whose
//! `Scan` reads the visible rows of its dataset the same way (E4-6.5c), the query-filter
//! `Filter`, `Deduplicate` when the branch joins and has metric components, and `Aggregate`
//! (E4-4). Two or more branches are combined by a `Stitch` (E4-5). Then come `Project`,
//! `Sort` and `Limit` (E4-9). A derived metric is expanded into the components of its inputs,
//! and its formula is computed in the final `Project` over them (DER-4.1). A context metric
//! reads its base at the result rows (E4-8.1) and in its own context query, planned like a
//! query and attached by a `Lookup` after the `Stitch` (E4-8.2, E4-8.7): [`context_queries`]
//! gives the context queries, and [`build_plan_with_contexts`] plans them with their branches.
//! `share_of_total` and `period_to_date` are planned: the branches of a `period_to_date`
//! context query repeat their rows with a `ToDate` (E4-8.5). `prior_period` comes in a later
//! issue and is an internal error here.
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
mod context;
mod derived_inputs;
mod dimensions;
mod error;
mod expansion;
mod explanation;
mod followed;
mod inline_output;
mod joins;
mod layout;
mod lookup;
mod metrics;
mod nodes;
mod output;
mod policy;
mod policy_visits;
mod population;
mod query_filter;
mod query_policies;
mod rewrite;
mod rows;
mod scan;
mod semi_additive;
mod slice;
mod snapshot_grain;
mod stitch;
mod to_date;
mod typing;
mod visibility;

pub use context::{ContextBranches, ContextQuery};
pub use error::{InvariantViolation, PlanError};
pub use explanation::{explanation, explanation_with_contexts};
pub use population::{PopulationRequest, plan_population};

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{NamedExpression, PlanNode};
use menura_representations::origin::Origin;
use menura_representations::paths::{Branch, BranchKind, Branches};
use menura_security::AdmittedSecurityContext;

use crate::layout::Layout;
use crate::nodes::aggregate;
use crate::to_date::{ToDatePlan, ToDateStep};

/// Builds the logical plan of `query` from its `branches`, for the user of `security`, over
/// `model` and the admitted `catalog` schema, which gives the type of every source column a
/// dataset metric reads (E4-2, CAT-4). The query has no context metric; a query with one is
/// planned by [`build_plan_with_contexts`].
///
/// # Errors
///
/// As [`build_plan_with_contexts`].
pub fn build_plan(
    branches: &Branches,
    query: &BoundQuery,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, PlanError> {
    build_plan_with_contexts(
        &PlanInputs {
            branches,
            query,
            contexts: &[],
        },
        security,
        model,
        catalog,
    )
}

/// The context queries of `query`, in the order of E4-8.8, each shared by the context metrics
/// with the same base and normalized modifier (E4-8.2). The caller resolves the branches of
/// each with E3 and passes them to [`build_plan_with_contexts`].
///
/// # Errors
///
/// [`PlanError::Invariant`] for a metric that [`build_plan_with_contexts`] rejects the same way.
pub fn context_queries(
    query: &BoundQuery,
    model: &InternalModel,
) -> Result<Vec<ContextQuery>, PlanError> {
    metrics::planned_metrics_with_contexts(query, model).map(|(_, contexts)| contexts)
}

/// What a plan is built from: the bound query, its branches, and its context queries with
/// their branches, in the order [`context_queries`] gives them.
#[derive(Debug, Clone, Copy)]
pub struct PlanInputs<'input> {
    /// The branches of the query (E3-4).
    pub branches: &'input Branches,
    /// The bound query.
    pub query: &'input BoundQuery,
    /// Its context queries, with their branches.
    pub contexts: &'input [ContextBranches<'input>],
}

/// Builds the logical plan of the query of `inputs`, for the user of `security`, over `model`
/// and the admitted `catalog` schema (E4-2, CAT-4). The rows of the query and of each context
/// query are planned alike (E4-4, E4-5, E4-8.2); one `Lookup` per context query, in order,
/// attaches its components to the result rows (E4-8.7), before the metric values are computed.
///
/// # Errors
///
/// [`PlanError::MissingAttribute`] (`ESEC-02`, E2-5 row 18) when an applicable row policy
/// needs an attribute `security` does not provide; then [`PlanError::MissingReferenceTime`]
/// (`EQRY-03`, E4-6.9b) when an applicable row policy reads the current date or time and the
/// query has no `reference_time`; then [`PlanError::MissingTimeKey`] (`E4-02`, E4-8.6) when a
/// `period_to_date` needs a time key the query does not have; [`PlanError::Invariant`] for an input that the stages
/// before E4 never produce, context queries other than those of [`context_queries`], or an
/// input that needs a later part of E4.
pub fn build_plan_with_contexts(
    inputs: &PlanInputs<'_>,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, PlanError> {
    let PlanInputs {
        branches,
        query,
        contexts,
    } = *inputs;
    let first = slice::first_branch(branches)?;
    let (metrics, expected) = metrics::planned_metrics_with_contexts(query, model)?;
    check_contexts(contexts, &expected)?;
    let outputs = metrics::metric_outputs(branches, &metrics)?;
    let home_origin = Origin::Model(slice::home_dataset(first, model)?.location.clone());
    let project_origin =
        metrics::first_metric_location(&metrics).map_or(home_origin, Origin::Model);
    let sources = RowSources {
        security,
        model,
        catalog,
    };
    let mut rows = query_rows(branches, query, &metrics, &sources, &project_origin, None)?;
    for (position, context) in contexts.iter().enumerate() {
        rows = context_lookup(rows, position, context, &sources)?;
    }
    let with_inputs =
        derived_inputs::compute_inputs(rows, &metrics::input_columns(&metrics), &project_origin)?;
    let projected = match inline_output::first_inline_metric(query) {
        Some(inline) => inline_output::project(
            query,
            with_inputs,
            outputs,
            &project_origin,
            Origin::Query(inline.clone()),
        )?,
        None => output::project(query, with_inputs, outputs, project_origin)?,
    };
    output::order_and_limit(query, projected)
}

/// What the rows of a query are read with: the user, the model and the catalog schema.
struct RowSources<'input> {
    security: &'input AdmittedSecurityContext,
    model: &'input InternalModel,
    catalog: &'input CatalogSchemaDocument,
}

/// Fails unless `contexts` are the context queries `expected`, in order, their `paths` aside:
/// the caller takes out the entries a context query does not use (E4-8.2).
fn check_contexts(
    contexts: &[ContextBranches<'_>],
    expected: &[ContextQuery],
) -> Result<(), PlanError> {
    let same = contexts.len() == expected.len()
        && contexts
            .iter()
            .zip(expected)
            .all(|(given, expected)| given.context.is_same_context(expected));
    if same {
        Ok(())
    } else {
        Err(PlanError::invariant(
            InvariantViolation::UnplannedContextQuery,
            &slice::query_root(),
        ))
    }
}

/// The rows of `query`, from its `branches`: the plan of each branch up to its `Aggregate`,
/// combined by a `Stitch` when there are several (E4-4, E4-5), whose origin is
/// `stitch_origin`. The branches of a `period_to_date` context query repeat their rows by
/// `to_date` (E4-8.5).
fn query_rows(
    branches: &Branches,
    query: &BoundQuery,
    metrics: &[metrics::PlannedMetric<'_>],
    sources: &RowSources<'_>,
    stitch_origin: &Origin,
    to_date: Option<&ToDatePlan>,
) -> Result<PlanNode, PlanError> {
    let RowSources {
        security,
        model,
        catalog,
    } = *sources;
    let has_reference_time = query.time().reference_time.is_some();
    query_policies::check_query_policies(branches, metrics, model, security, has_reference_time)?;
    let mut plans = branches
        .as_slice()
        .iter()
        .map(|branch| {
            let source = BranchInputs {
                branch,
                query,
                metrics,
                to_date,
            };
            branch_plan(&source, security, model, catalog)
        })
        .collect::<Result<Vec<_>, _>>()?;
    snapshot_grain::check_snapshot_grains(query, &snapshot_axes(branches, metrics, model)?)?;
    match plans.len() {
        0 => Err(slice::no_branch()),
        1 => plans.pop().ok_or_else(slice::no_branch),
        _ => stitch::stitch(plans, stitch_origin.clone()),
    }
}

/// `rows` with the components of the context query at `position` attached by its `Lookup`
/// (E4-8.2, E4-8.7): the context query is planned like a query, with its own branches. A
/// `period_to_date` without its time key is `E4-02`, after the row policies of the context
/// query are checked (E2-5 rows 18 and 19).
fn context_lookup(
    rows: PlanNode,
    position: usize,
    context: &ContextBranches<'_>,
    sources: &RowSources<'_>,
) -> Result<PlanNode, PlanError> {
    let query = context.context.query();
    let metrics = metrics::planned_metrics(query, sources.model)?;
    let to_date = match context.context.to_date() {
        Ok(to_date) => to_date,
        Err(location) => {
            let has_reference_time = query.time().reference_time.is_some();
            query_policies::check_query_policies(
                context.branches,
                &metrics,
                sources.model,
                sources.security,
                has_reference_time,
            )?;
            return Err(PlanError::MissingTimeKey(location.clone()));
        }
    };
    let first = slice::first_branch(context.branches)?;
    let home_origin = Origin::Model(slice::home_dataset(first, sources.model)?.location.clone());
    let stitch_origin = metrics::first_metric_location(&metrics).map_or(home_origin, Origin::Model);
    let value = query_rows(
        context.branches,
        query,
        &metrics,
        sources,
        &stitch_origin,
        to_date,
    )?;
    lookup::lookup(rows, value, position, context.context)
}

/// One branch to plan: the branch, its query and metrics, and its `ToDate` in a
/// `period_to_date` context query.
struct BranchInputs<'input> {
    branch: &'input Branch,
    query: &'input BoundQuery,
    metrics: &'input [metrics::PlannedMetric<'input>],
    to_date: Option<&'input ToDatePlan>,
}

/// The plan of a branch, up to its `Aggregate` (E4-4); with a `ToDate`, its keys read the
/// period t (E4-8.5).
fn branch_plan(
    inputs: &BranchInputs<'_>,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<PlanNode, PlanError> {
    let BranchInputs {
        branch,
        query,
        metrics,
        to_date,
    } = *inputs;
    let home = slice::home_dataset(branch, model)?;
    let home_origin = Origin::Model(home.location.clone());
    let layout = Layout::new(branch, home, model)?;
    let dimensions::BranchKeys { keys, computed } = dimensions::dimension_keys(query, &layout)?;
    let step = to_date_step(to_date, branch, &layout, &keys, &home_origin)?;
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
    if let BranchKind::SemiAdditive { .. } = branch.kind {
        let (metric, declaration) = metrics::semi_additive_metric(branch, metrics)?;
        return semi_additive::semi_additive_plan(&semi_additive::SemiAdditiveBranch {
            source,
            query,
            metric,
            declaration,
            keys,
            computed,
            components,
        });
    }
    let request = rows::RowsRequest {
        keys: &keys,
        computed: computed.as_ref(),
        components: &components,
        fields: &[],
        predicate: query_filter::query_predicate(query, &layout)?,
        replaced_home: None,
        to_date: step.as_ref(),
    };
    let node = rows::branch_rows(&source, request)?;
    let keys = step.map_or(keys, |step| step.keys);
    aggregate(node, keys, components, &home_origin)
}

/// The `ToDate` of `branch`, whose layout is `layout` and keys `keys`, in a `period_to_date`
/// context query (E4-8.5). A semi-additive branch has none, since E0 rejects a
/// `period_to_date` over a semi-additive base (`E0C-65`).
fn to_date_step(
    to_date: Option<&ToDatePlan>,
    branch: &Branch,
    layout: &Layout<'_>,
    keys: &[NamedExpression],
    origin: &Origin,
) -> Result<Option<ToDateStep>, PlanError> {
    let Some(plan) = to_date else {
        return Ok(None);
    };
    let violation = match branch.kind {
        BranchKind::SemiAdditive { .. } => InvariantViolation::ContextMetric,
        BranchKind::Plain => match plan.step(layout, keys) {
            Ok(step) => return Ok(Some(step)),
            Err(violation) => violation,
        },
    };
    Err(PlanError::invariant(violation, origin))
}

/// The time `over` of each semi-additive branch of `branches`, in branch order, for the check
/// of `E4-03` (ADD-4.9).
fn snapshot_axes<'model>(
    branches: &'model Branches,
    metrics: &[metrics::PlannedMetric<'model>],
    model: &'model InternalModel,
) -> Result<Vec<snapshot_grain::SnapshotAxis<'model>>, PlanError> {
    let mut axes = Vec::new();
    for branch in branches.as_slice() {
        let BranchKind::SemiAdditive { .. } = branch.kind else {
            continue;
        };
        let (metric, declaration) = metrics::semi_additive_metric(branch, metrics)?;
        let Some(snapshot_grain) = declaration.snapshot_grain else {
            continue;
        };
        let home = slice::home_dataset(branch, model)?;
        let over_type = semi_additive::over_type(declaration, home).map_err(|violation| {
            PlanError::invariant(violation, &Origin::Model(metric.location.clone()))
        })?;
        axes.push(snapshot_grain::SnapshotAxis {
            over: &declaration.over,
            over_type,
            snapshot_grain,
        });
    }
    Ok(axes)
}
