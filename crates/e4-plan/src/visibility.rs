//! The visible rows V(D) of a dataset over its `Scan` (E4-6.1, E4-6.5, E4-6.5a, E4-6.5c): a
//! `Filter` for its own applicable policies, directly over the `Scan`, then one `SemiJoin` per
//! chain of relationships that E4-6.5 follows.
//!
//! A `SemiJoin` keeps a row when at least one row is related along the first step of its
//! `path` and every such row matches the rest of the path; the empty rest matches when the
//! `predicate` holds. A NULL foreign key matches nothing. Its `predicate` is the row filter of
//! the last dataset of the path for the user, `TRUE` when that dataset has no policy, over the
//! fields of that dataset by normalized name. The `SemiJoin` nodes of all chains together give
//! the recursive V(D) of E4-6.5.

use menura_model::expression::{Expression, Literal};
use menura_model::internal_model::{Dataset, InternalModel, NormalizedName};
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::names::ColumnPrefix;
use menura_representations::origin::Origin;
use menura_representations::paths::JoinStep;

use crate::columns::{column_reference, over_scan, read_fields};
use crate::error::{InvariantViolation, PlanError};
use crate::policy_visits::RowFilters;
use crate::propagation_chains::propagation_chains;
use crate::rewrite::rewrite;

/// The fields of `dataset` its visible rows read: those of its own row filter, then the
/// `from_columns` of the first step of every chain, in first-read order.
pub(crate) fn visibility_fields(
    dataset: &Dataset,
    model: &InternalModel,
    filters: &RowFilters,
) -> Result<Vec<NormalizedName>, PlanError> {
    let mut fields = Vec::new();
    if let Some(own) = filters.of(&dataset.name) {
        let predicate = over_scan(&own.filter, None).map_err(|violation| {
            PlanError::invariant(violation, &Origin::Model(own.location.clone()))
        })?;
        read_fields(&predicate, &mut fields);
    }
    for chain in propagation_chains(model, &dataset.name) {
        if let [step] = chain.as_slice() {
            fields.extend(first_step_columns(step, model, dataset)?.iter().cloned());
        }
    }
    Ok(fields)
}

/// The visible rows of `dataset` over its `scan`, whose columns are the fields of `dataset`
/// under `prefix` (no prefix for the home dataset).
///
/// # Errors
///
/// An internal error for a relationship the model does not have or a column name that is
/// empty.
pub(crate) fn visible_rows(
    scan: PlanNode,
    dataset: &Dataset,
    prefix: Option<&ColumnPrefix>,
    model: &InternalModel,
    filters: &RowFilters,
) -> Result<PlanNode, PlanError> {
    let mut node = scan;
    if let Some(own) = filters.of(&dataset.name) {
        let origin = Origin::Model(own.location.clone());
        let predicate = over_scan(&own.filter, None)
            .and_then(|predicate| prefixed(&predicate, prefix))
            .map_err(|violation| PlanError::invariant(violation, &origin))?;
        node = wrap(node, origin, |input| Operation::Filter { input, predicate });
    }
    for path in propagation_chains(model, &dataset.name) {
        let Some(last) = path.last() else {
            continue;
        };
        let (predicate, origin) = related_predicate(last, model, filters)?;
        node = wrap(node, origin, |input| Operation::SemiJoin {
            input,
            path,
            predicate,
        });
    }
    Ok(node)
}

/// The predicate of a `SemiJoin` whose path ends with `last`, over the fields of the dataset
/// `last` reaches, and its origin: the policies of that dataset, or the relationship of `last`
/// when it has none.
fn related_predicate(
    last: &JoinStep,
    model: &InternalModel,
    filters: &RowFilters,
) -> Result<(Expression, Origin), PlanError> {
    if let Some(related) = filters.of(&last.to) {
        let origin = Origin::Model(related.location.clone());
        let predicate = over_scan(&related.filter, None)
            .map_err(|violation| PlanError::invariant(violation, &origin))?;
        return Ok((predicate, origin));
    }
    let relationship = model.relationship(&last.relationship).ok_or_else(|| {
        PlanError::invariant(
            InvariantViolation::UnknownRelationship,
            &crate::slice::query_root(),
        )
    })?;
    Ok((
        Expression::Literal(Literal::Boolean(true)),
        Origin::Model(relationship.location.clone()),
    ))
}

/// The `from_columns` of `step`, which leaves `dataset`.
fn first_step_columns<'model>(
    step: &JoinStep,
    model: &'model InternalModel,
    dataset: &Dataset,
) -> Result<&'model [NormalizedName], PlanError> {
    model
        .relationship(&step.relationship)
        .map(|relationship| relationship.from_columns.as_slice())
        .ok_or_else(|| {
            PlanError::invariant(
                InvariantViolation::UnknownRelationship,
                &Origin::Model(dataset.location.clone()),
            )
        })
}

/// `predicate`, over the unprefixed `Scan` columns, with every column under `prefix`.
fn prefixed(
    predicate: &Expression,
    prefix: Option<&ColumnPrefix>,
) -> Result<Expression, InvariantViolation> {
    let Some(prefix) = prefix else {
        return Ok(predicate.clone());
    };
    rewrite(predicate, &mut |node| match node {
        Expression::Reference(reference) => {
            column_reference(prefix.column(reference.name.text()).as_str()).map(Some)
        }
        _ => Ok(None),
    })
}

/// The node of `operation` over `input`, with the schema of `input`.
fn wrap(
    input: PlanNode,
    origin: Origin,
    operation: impl FnOnce(Box<PlanNode>) -> Operation,
) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(operation(Box::new(input)), schema, origin)
}
