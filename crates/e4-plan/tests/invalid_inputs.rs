//! Inputs the stages before E4 never produce are internal errors, one per variant.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::query::{Direction, NullsOrder};
use menura_e4_plan::{InvariantViolation, PlanError};
use menura_model::internal_model::{InternalModel, Metric, RowPolicy};
use menura_representations::bound_query::{BoundQuery, BoundQueryParts};
use menura_representations::logical_plan::PlanNode;
use menura_representations::origin::Origin;
use menura_representations::paths::Branches;
use support::{
    POLICY_POINTER, anyone, branches, build_plan, date, date_range, dimension, located,
    metric_reference, metrics, model, model_with, orders, policy, query,
};

fn violation(result: Result<PlanNode, PlanError>) -> InvariantViolation {
    match result {
        Err(PlanError::Invariant { violation, .. }) => violation,
        other => panic!("expected an internal error, got {other:?}"),
    }
}

fn count_query(parts: BoundQueryParts) -> BoundQuery {
    query(
        vec![dimension("REGION", None, 0)],
        vec![metric_reference("ORDER_COUNT", 0)],
        parts,
    )
}

fn plan(model: &InternalModel, parts: BoundQueryParts) -> Result<PlanNode, PlanError> {
    build_plan(&branches(&[1]), &count_query(parts), &anyone(), model)
}

fn with_metric(changed: Metric) -> InternalModel {
    let mut all = metrics();
    all.retain(|candidate| candidate.name != changed.name);
    all.push(changed);
    all.sort_by(|left, right| left.name.cmp(&right.name));
    model_with(vec![orders(Vec::new())], all)
}

fn order_count() -> Metric {
    metrics()
        .into_iter()
        .find(|candidate| candidate.name.as_str() == "ORDER_COUNT")
        .expect("ORDER_COUNT")
}

#[test]
fn vs_1_no_branch_is_an_internal_error() {
    let none = Branches::new(Vec::new()).expect("no branch");
    let result = build_plan(
        &none,
        &count_query(BoundQueryParts::default()),
        &anyone(),
        &model(Vec::new()),
    );
    assert_eq!(violation(result), InvariantViolation::NoBranch);
}

#[test]
fn vs_1_a_field_without_a_type_is_an_internal_error() {
    let mut home = orders(Vec::new());
    for field in &mut home.fields {
        field.typed = None;
    }
    let model = model_with(vec![home], metrics());
    let result = plan(&model, BoundQueryParts::default());
    assert_eq!(violation(result), InvariantViolation::UntypedField);
}

#[test]
fn vs_1_a_metric_without_a_type_is_an_internal_error() {
    let mut untyped = order_count();
    untyped.typed = None;
    let result = plan(&with_metric(untyped), BoundQueryParts::default());
    assert_eq!(violation(result), InvariantViolation::UntypedExpression);
}

#[test]
fn vs_1_a_metric_the_model_does_not_have_is_an_internal_error() {
    let missing = model_with(
        vec![orders(Vec::new())],
        metrics()
            .into_iter()
            .filter(|candidate| candidate.name.as_str() != "ORDER_COUNT")
            .collect(),
    );
    let result = plan(&missing, BoundQueryParts::default());
    assert_eq!(violation(result), InvariantViolation::UnknownMetric);
}

#[test]
fn vs_1_aggregates_that_do_not_match_the_components_are_an_internal_error() {
    let mut mismatched = order_count();
    if let Some(classification) = &mut mismatched.classification {
        let first = classification.components[0].clone();
        classification.components.push(first);
    }
    let result = build_plan(
        &branches(&[2]),
        &count_query(BoundQueryParts::default()),
        &anyone(),
        &with_metric(mismatched),
    );
    assert_eq!(violation(result), InvariantViolation::ComponentMismatch);
}

#[test]
fn vs_1_a_date_range_on_a_text_field_is_an_internal_error() {
    let parts = BoundQueryParts {
        date_range: Some(date_range("REGION", date(2026, 1, 1), date(2026, 1, 31))),
        ..BoundQueryParts::default()
    };
    let result = plan(&model(Vec::new()), parts);
    assert_eq!(violation(result), InvariantViolation::UnplannableDateRange);
}

#[test]
fn vs_1_an_ordering_term_beyond_the_output_is_an_internal_error() {
    let parts = BoundQueryParts {
        order: vec![support::order(9, Direction::Asc, NullsOrder::Last, 0)],
        ..BoundQueryParts::default()
    };
    let result = plan(&model(Vec::new()), parts);
    assert_eq!(violation(result), InvariantViolation::UnknownOrderColumn);
}

#[test]
fn vs_1_an_empty_column_name_is_an_internal_error() {
    let mut home = orders(Vec::new());
    let mut empty = home.fields[0].clone();
    empty.name = support::name("");
    home.fields.insert(0, empty);
    let model = model_with(vec![home], metrics());
    let bound = query(
        vec![dimension("", None, 0)],
        vec![metric_reference("ORDER_COUNT", 0)],
        BoundQueryParts::default(),
    );
    let result = build_plan(&branches(&[1]), &bound, &anyone(), &model);
    assert_eq!(violation(result), InvariantViolation::EmptyName);
}

#[test]
fn vs_1_an_unparsed_policy_condition_is_an_internal_error_at_the_policy() {
    let mut unparsed: RowPolicy = policy("region_scope", "TRUE", None, None);
    unparsed.condition = None;
    let result = plan(&model(vec![unparsed]), BoundQueryParts::default());
    match result {
        Err(PlanError::Invariant { violation, origin }) => {
            assert_eq!(violation, InvariantViolation::UnparsedCondition);
            assert_eq!(origin, Origin::Model(located(POLICY_POINTER)));
        }
        other => panic!("expected an internal error, got {other:?}"),
    }
}
