//! Stage E4: builds the logical plan (E4-1, E4-2).
//!
//! In the 0.3.0 slice (VS-1) a query has one branch on one dataset, with that dataset's own
//! row policies. [`build_plan`] builds its plan: `Scan`, the row-policy `Filter`, the
//! query-filter `Filter`, `Aggregate`, `Project`, then `Sort` and `Limit` (E4-4, E4-9). Joins,
//! propagation, `Deduplicate`, `Stitch`, additivity and context modifiers come in later
//! versions; an input that needs them is an internal error, because E2 row 12c rules it out.
//!
//! The plan follows the conventions shared with E6 (issue #251): a `Scan` projects fields of
//! its dataset by normalized name, every other node reads the columns of its input by
//! unqualified reference, the query-filter `Filter` comes from the query, and the row-policy
//! `Filter`, the `Aggregate` and the final `Project` come from the model, so their literals
//! are model constants.

#![forbid(unsafe_code)]

mod columns;
mod dimensions;
mod error;
mod metrics;
mod output;
mod policy;
mod query_filter;
mod rewrite;
mod scan;
mod slice;
mod typing;

pub use error::{InvariantViolation, PlanError};

use menura_model::expression::Expression;
use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode,
};
use menura_representations::origin::Origin;
use menura_representations::paths::Branches;
use menura_security::AdmittedSecurityContext;

use crate::columns::read_fields;
use crate::typing::typed_column;

/// Builds the logical plan of `query` from its `branches`, for the user of `security`, over
/// `model` (E4-2).
///
/// # Errors
///
/// [`PlanError::MissingAttribute`] (`ESEC-02`, E2-5 row 18) when an applicable row policy
/// needs an attribute `security` does not provide; [`PlanError::Invariant`] for an input
/// outside the 0.3.0 slice, which the stages before E4 never produce.
pub fn build_plan(
    branches: &Branches,
    query: &BoundQuery,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
) -> Result<PlanNode, PlanError> {
    let branch = slice::single_branch(branches)?;
    let home = slice::home_dataset(branch, model)?;
    let metrics = metrics::planned_metrics(query, model)?;
    let keys = dimensions::dimension_keys(query, home)?;
    let components = metrics::branch_components(branch, &metrics, home)?;
    let outputs = metrics::metric_outputs(branch, &metrics)?;
    let query_predicate = query_filter::query_predicate(query, home)?;
    let policy_predicate = policy::policy_predicate(home, security)?;

    let read = fields_read(
        &keys,
        &components,
        [
            query_predicate.as_ref().map(|(predicate, _)| predicate),
            policy_predicate.as_ref().map(|(predicate, _)| predicate),
        ],
    );
    let mut node = scan::scan(home, &read, Vec::new())?;
    if let Some((predicate, location)) = policy_predicate {
        node = filter(node, predicate, Origin::Model(location));
    }
    if let Some((predicate, location)) = query_predicate {
        node = filter(node, predicate, Origin::Query(location));
    }
    let home_origin = Origin::Model(home.location.clone());
    let aggregate = aggregate(node, keys, components, &home_origin)?;
    let project_origin =
        metrics::first_metric_location(&metrics).map_or(home_origin, Origin::Model);
    let projected = output::project(query, aggregate, outputs, project_origin)?;
    output::order_and_limit(query, projected)
}

/// The fields the keys, the components and the predicates read.
fn fields_read(
    keys: &[NamedExpression],
    components: &[AggregateComponent],
    predicates: [Option<&Expression>; 2],
) -> Vec<NormalizedName> {
    let mut read = Vec::new();
    for key in keys {
        read_fields(&key.expression, &mut read);
    }
    for component in components {
        read_fields(&component_expression(component), &mut read);
    }
    for predicate in predicates.into_iter().flatten() {
        read_fields(predicate, &mut read);
    }
    read
}

/// The `Filter` of `input` by `predicate`: it keeps the input's schema.
fn filter(input: PlanNode, predicate: Expression, origin: Origin) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Filter {
            input: Box::new(input),
            predicate,
        },
        schema,
        origin,
    )
}

/// The `Aggregate` of `input` keyed by `keys`, with `components` (E4-4 step 6).
fn aggregate(
    input: PlanNode,
    keys: Vec<NamedExpression>,
    components: Vec<AggregateComponent>,
    origin: &Origin,
) -> Result<PlanNode, PlanError> {
    let fail = |violation| PlanError::invariant(violation, origin);
    let mut columns = Vec::new();
    for key in &keys {
        columns
            .push(typed_column(key.name.clone(), &key.expression, input.schema()).map_err(fail)?);
    }
    for component in &components {
        let expression = component_expression(component);
        columns
            .push(typed_column(component.name.clone(), &expression, input.schema()).map_err(fail)?);
    }
    Ok(PlanNode::new(
        Operation::Aggregate {
            input: Box::new(input),
            keys,
            components,
        },
        OutputSchema::new(columns),
        origin.clone(),
    ))
}

/// The aggregate expression of `component`.
fn component_expression(component: &AggregateComponent) -> Expression {
    Expression::Aggregate {
        call: component.call.clone(),
        filter: component.filter.clone().map(Box::new),
    }
}
