//! Inputs outside the 0.3.0 slice, which E2 row 12c rules out, are internal errors (VS-1).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;
use menura_e4_plan::{InvariantViolation, PlanError};
use menura_model::internal_model::{
    ContextMetric, ContextModifier, DatasetReach, DeclaredAdditivity, DerivedMetric, InternalModel,
    Metric, NullHandling, PathCount, ReachedDataset, SemiAdditive, SnapshotAggregation,
};
use menura_representations::bound_query::{
    BoundDimension, BoundDimensionKind, BoundMetric, BoundMetricKind, BoundQuery, BoundQueryParts,
};
use menura_representations::paths::{Branch, Branches, ComponentKey, JoinTree};
use support::{
    anyone, branches, build_plan, dimension, field_path, metric, metric_reference, metrics, model,
    model_with, name, orders, policy, query,
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
        components: vec![ComponentKey {
            metric: 0,
            component: 0,
        }],
        join_tree,
    }
}

fn plan_with_metric(model_metric: Metric) -> InvariantViolation {
    let mut all = metrics();
    all.retain(|candidate| candidate.name != model_metric.name);
    all.push(model_metric);
    all.sort_by(|left, right| left.name.cmp(&right.name));
    let model = model_with(vec![orders(Vec::new())], all);
    violation(build_plan(
        &branches(&[1]),
        &count_query(),
        &anyone(),
        &model,
    ))
}

fn order_count() -> Metric {
    metric(
        "ORDER_COUNT",
        "COUNT(*)",
        menura_expressions::typing::big_integer(),
    )
}

#[test]
fn vs_1_more_than_one_branch_is_an_internal_error() {
    let several = Branches::new(vec![
        branch("ORDERS", JoinTree::default()),
        branch("RETURNS", JoinTree::default()),
    ])
    .expect("two homes");
    let result = build_plan(&several, &count_query(), &anyone(), &model(Vec::new()));
    assert_eq!(violation(result), InvariantViolation::SeveralBranches);
}

#[test]
fn vs_1_policy_propagation_is_an_internal_error() {
    let mut customers = orders(vec![policy("customer_scope", "TRUE", None, None)]);
    customers.name = name("CUSTOMERS");
    let base = model_with(vec![customers, orders(Vec::new())], metrics());
    let mut parts = base.parts().clone();
    parts.graph.reachability = vec![DatasetReach {
        dataset: name("ORDERS"),
        reached: vec![ReachedDataset {
            dataset: name("CUSTOMERS"),
            paths: PathCount::One,
        }],
    }];
    let model = InternalModel::new(parts, base.hash().clone());
    let result = build_plan(&branches(&[1]), &count_query(), &anyone(), &model);
    assert_eq!(violation(result), InvariantViolation::Propagation);
}

#[test]
fn vs_1_a_derived_metric_is_an_internal_error() {
    let mut derived = order_count();
    derived.derived = Some(DerivedMetric {
        inputs: vec![name("TOTAL_AMOUNT")],
        on_division_by_zero: None,
    });
    assert_eq!(plan_with_metric(derived), InvariantViolation::DerivedMetric);
}

fn semi_additive(population: Option<&str>) -> DeclaredAdditivity {
    DeclaredAdditivity::SemiAdditive(SemiAdditive {
        over: field_path("ORDER_DATE"),
        entity: vec![field_path("REGION")],
        aggregation: SnapshotAggregation::Minimum,
        nulls: NullHandling::Keep,
        snapshot_grain: None,
        population: population.map(name),
        missing: None,
        unexpected: None,
    })
}

#[test]
fn vs_1_an_additivity_metric_is_an_internal_error() {
    for declared in [DeclaredAdditivity::NonAdditive, semi_additive(None)] {
        let mut declaring = order_count();
        declaring.declared_additivity = Some(declared);
        assert_eq!(
            plan_with_metric(declaring),
            InvariantViolation::AdditivityMetric
        );
    }
}

#[test]
fn vs_1_a_population_metric_is_an_internal_error() {
    let mut declaring = order_count();
    declaring.declared_additivity = Some(semi_additive(Some("CUSTOMER_BASE")));
    assert_eq!(
        plan_with_metric(declaring),
        InvariantViolation::PopulationMetric
    );
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
fn vs_1_an_inline_metric_is_an_internal_error() {
    let inline = BoundMetric {
        kind: BoundMetricKind::Inline(support::tree("AGG(order_count)")),
        ..metric_reference("ORDER_COUNT", 0)
    };
    let bound = query(Vec::new(), vec![inline], BoundQueryParts::default());
    let result = build_plan(&branches(&[1]), &bound, &anyone(), &model(Vec::new()));
    assert_eq!(violation(result), InvariantViolation::InlineMetric);
}

#[test]
fn vs_1_an_expression_dimension_is_an_internal_error() {
    let expression = BoundDimension {
        kind: BoundDimensionKind::Expression {
            dataset: name("ORDERS"),
            expression: support::tree("UPPER(region)"),
        },
        ..dimension("REGION", None, 0)
    };
    let bound = query(vec![expression], Vec::new(), BoundQueryParts::default());
    let result = build_plan(&branches(&[]), &bound, &anyone(), &model(Vec::new()));
    assert_eq!(violation(result), InvariantViolation::ExpressionDimension);
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
