//! Inputs outside the 0.3.0 slice, which E2 row 12c rules out, are internal errors (VS-1).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;
use menura_e4_plan::{InvariantViolation, PlanError};
use menura_model::internal_model::{ContextMetric, ContextModifier, DeclaredAdditivity, Metric};
use menura_representations::bound_query::{BoundDimensionKind, BoundQuery, BoundQueryParts};
use menura_representations::paths::{Branch, BranchKind, Branches, ComponentKey, JoinTree};
use support::{
    anyone, branches, build_plan, dimension, field_path, metric, metric_reference, metrics, model,
    model_with, name, orders, query,
};

fn count_query() -> BoundQuery {
    query(
        vec![dimension("REGION", None, 0)],
        vec![metric_reference("ORDER_COUNT", 0)],
        BoundQueryParts::default(),
    )
}

fn violation(
    result: Result<menura_representations::logical_plan::PlanNode, PlanError>,
) -> InvariantViolation {
    match result {
        Err(PlanError::Invariant { violation, .. }) => violation,
        other => panic!("expected an internal error, got {other:?}"),
    }
}

fn branch(home: &str, join_tree: JoinTree) -> Branch {
    Branch {
        home: name(home),
        kind: BranchKind::Plain,
        components: vec![ComponentKey {
            metric: 0,
            component: 0,
        }],
        join_tree,
    }
}

fn plan_of(
    model_metric: Metric,
) -> Result<menura_representations::logical_plan::PlanNode, PlanError> {
    let mut all = metrics();
    all.retain(|candidate| candidate.name != model_metric.name);
    all.push(model_metric);
    all.sort_by(|left, right| left.name.cmp(&right.name));
    let model = model_with(vec![orders(Vec::new())], all);
    build_plan(&branches(&[1]), &count_query(), &anyone(), &model)
}

fn plan_with_metric(model_metric: Metric) -> InvariantViolation {
    violation(plan_of(model_metric))
}

fn order_count() -> Metric {
    metric(
        "ORDER_COUNT",
        "COUNT(*)",
        menura_expressions::typing::big_integer(),
    )
}

#[test]
fn e4_7_9_a_non_additive_metric_plans_as_its_deduced_additivity() {
    let mut declaring = order_count();
    declaring.declared_additivity = Some(DeclaredAdditivity::NonAdditive);
    assert_eq!(plan_of(declaring), plan_of(order_count()));
}

#[test]
fn vs_1_a_context_metric_is_an_internal_error() {
    let mut context = order_count();
    context.context = Some(ContextMetric {
        base: name("TOTAL_AMOUNT"),
        modifier: ContextModifier::PriorPeriod {
            time_dimension: field_path("ORDER_DATE"),
            grain: Grain::Year,
            offset: NonZeroU64::MIN,
        },
    });
    assert_eq!(plan_with_metric(context), InvariantViolation::ContextMetric);
}

#[test]
fn vs_1_a_field_of_another_dataset_is_an_internal_error() {
    let mut foreign = dimension("REGION", None, 0);
    foreign.kind = BoundDimensionKind::Field {
        field: menura_model::internal_model::FieldPath {
            dataset: name("CUSTOMERS"),
            field: name("REGION"),
        },
        time_grain: None,
        time_grain_origin: None,
    };
    let bound = query(vec![foreign], Vec::new(), BoundQueryParts::default());
    let result = build_plan(&branches(&[]), &bound, &anyone(), &model(Vec::new()));
    assert_eq!(violation(result), InvariantViolation::ForeignField);
}

#[test]
fn vs_1_a_component_the_metric_does_not_have_is_an_internal_error() {
    let result = build_plan(
        &branches(&[2]),
        &count_query(),
        &anyone(),
        &model(Vec::new()),
    );
    assert_eq!(violation(result), InvariantViolation::UnknownComponent);
}

#[test]
fn vs_1_an_unknown_home_dataset_is_an_internal_error() {
    let unknown = Branches::new(vec![branch("RETURNS", JoinTree::default())]).expect("one home");
    let result = build_plan(&unknown, &count_query(), &anyone(), &model(Vec::new()));
    assert_eq!(violation(result), InvariantViolation::UnknownDataset);
}

#[test]
fn vs_1_an_internal_error_has_no_specification_diagnostic() {
    let result = build_plan(
        &branches(&[2]),
        &count_query(),
        &anyone(),
        &model(Vec::new()),
    );
    let error = result.expect_err("an internal error");
    assert!(error.to_diagnostic().is_none());
}
