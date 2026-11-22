//! Derived metrics: their inputs computed as components, the formula applied in the final
//! `Project` (DER-2, DER-4, E4-5.0c).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::Grain;
use menura_e4_plan::{InvariantViolation, PlanError};
use menura_expressions::typing::decimal;
use menura_model::expression::{
    AggregateCall, BinaryOperator, Expression, IntegerLiteral, Literal, SimpleAggregate,
};
use menura_model::internal_model::{
    ContextMetric, ContextModifier, DeclaredAdditivity, DivisionByZero, FieldPath, InternalModel,
};
use menura_representations::logical_plan::{DerivedFormula, NamedExpression, Operation, PlanNode};
use support::derived::{changed_model, derived_model, derived_plan, float, plan_over};
use support::plan::{column, column_names, column_reference, kind, kinds, node};
use support::stitch::{branch, region, stitch_parts};
use support::{name, tree};

fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn ratio(numerator: &str, denominator: &str) -> Expression {
    binary(
        BinaryOperator::Divide,
        column_reference(numerator),
        column_reference(denominator),
    )
}

fn metric_expression(plan: &PlanNode, output_name: &str) -> Expression {
    match plan.operation() {
        Operation::Project { expressions, .. } => expressions
            .iter()
            .find(|named| named.name.as_str() == output_name)
            .map(|named| named.expression.clone())
            .expect("the metric column"),
        _ => panic!("not a Project"),
    }
}

fn aggregate_calls(aggregate: &PlanNode) -> Vec<(&str, &AggregateCall)> {
    match aggregate.operation() {
        Operation::Aggregate { components, .. } => components
            .iter()
            .map(|component| (component.name.as_str(), &component.call))
            .collect(),
        _ => panic!("not an Aggregate"),
    }
}

fn is_sum(call: &AggregateCall) -> bool {
    matches!(
        call,
        AggregateCall::Simple {
            function: SimpleAggregate::Sum,
            ..
        }
    )
}

#[test]
fn der_2_1_a_ratio_of_two_metrics_of_one_dataset_is_one_branch_and_a_project() {
    let plan = derived_plan(
        vec![branch("BUDGETS", &[(0, 0), (0, 1)], true)],
        vec![region()],
        &["AVERAGE_BUDGET"],
    );

    assert_eq!(kinds(&plan)[..2], ["Project", "Aggregate"]);
    let calls = aggregate_calls(node(&plan, "Aggregate"));
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].0, "metric_1_component_1");
    assert!(is_sum(calls[0].1));
    assert_eq!(
        calls[1],
        ("metric_1_component_2", &AggregateCall::CountRows)
    );
    assert_eq!(
        metric_expression(&plan, "average_budget"),
        ratio("metric_1_component_1", "metric_1_component_2")
    );
}

#[test]
fn der_4_1_the_inputs_are_aggregated_first_and_the_formula_is_applied_after() {
    let plan = derived_plan(
        vec![branch("BUDGETS", &[(0, 0), (0, 1)], false)],
        Vec::new(),
        &["AVERAGE_BUDGET"],
    );

    let Operation::Project { input, .. } = plan.operation() else {
        panic!("not a Project");
    };
    assert_eq!(kind(input), "Aggregate");
    assert!(aggregate_calls(input).iter().all(|(_, call)| !matches!(
        call,
        AggregateCall::Simple {
            function: SimpleAggregate::Average,
            ..
        }
    )));
}

#[test]
fn der_4_3_a_ratio_across_two_datasets_stitches_one_branch_per_input() {
    let plan = derived_plan(
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(0, 1)], true),
        ],
        vec![region()],
        &["SALES_TO_BUDGET"],
    );

    assert_eq!(kinds(&plan)[..2], ["Project", "Stitch"]);
    let (branches, keys, columns) = stitch_parts(&plan);
    assert_eq!(branches.len(), 2);
    assert_eq!(keys, ["dimension_1"]);
    let names: Vec<_> = columns.iter().map(|column| column.name.as_str()).collect();
    assert_eq!(names, ["metric_1_component_2", "metric_1_component_1"]);
    assert_eq!(
        metric_expression(&plan, "sales_to_budget"),
        ratio("metric_1_component_1", "metric_1_component_2")
    );
}

#[test]
fn der_5_1_3_the_project_column_has_the_e0f_5a_type_of_the_derived_metric() {
    let plan = derived_plan(
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(0, 1)], true),
        ],
        vec![region()],
        &["SALES_TO_BUDGET"],
    );

    assert_eq!(
        plan.schema().columns()[1],
        column("sales_to_budget", float(), true)
    );
}

fn named(plan: &PlanNode, output_name: &str) -> NamedExpression {
    match plan.operation() {
        Operation::Project { expressions, .. } => expressions
            .iter()
            .find(|named| named.name.as_str() == output_name)
            .cloned()
            .expect("the column"),
        _ => panic!("not a Project"),
    }
}

fn formula(metric: &str, rule: DivisionByZero) -> DerivedFormula {
    DerivedFormula {
        metric: name(metric),
        on_division_by_zero: rule,
    }
}

fn percent_plan(model: &InternalModel) -> PlanNode {
    plan_over(
        model,
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(0, 1)], true),
        ],
        vec![region()],
        &["SALES_TO_BUDGET_PERCENT"],
    )
    .expect("a plan")
}

fn hundred() -> Expression {
    Expression::Literal(Literal::Integer(
        IntegerLiteral::new("100").expect("a literal"),
    ))
}

#[test]
fn der_2_4_a_derived_input_is_computed_in_a_lower_project() {
    let plan = percent_plan(&derived_model());

    assert_eq!(kinds(&plan)[..3], ["Project", "Project", "Stitch"]);
    let lower = plan.inputs()[0];
    assert_eq!(
        column_names(lower),
        [
            "dimension_1",
            "metric_1_component_2",
            "metric_1_component_1",
            "metric_1_input_1"
        ]
    );
    assert_eq!(
        named(lower, "metric_1_input_1").expression,
        ratio("metric_1_component_1", "metric_1_component_2")
    );
    assert_eq!(
        named(lower, "dimension_1").expression,
        column_reference("dimension_1")
    );
    assert_eq!(
        metric_expression(&plan, "sales_to_budget_percent"),
        binary(
            BinaryOperator::Multiply,
            column_reference("metric_1_input_1"),
            hundred()
        )
    );
    assert_eq!(
        lower.schema().columns()[3],
        column("metric_1_input_1", float(), true)
    );
}

#[test]
fn der_3_1_each_formula_carries_the_rule_of_its_own_derived_metric() {
    let model = changed_model("SALES_TO_BUDGET_PERCENT", |metric| {
        metric.expression = Some(tree("AGG(sales_to_budget) / 100"));
        if let Some(derived) = metric.derived.as_mut() {
            derived.on_division_by_zero = Some(DivisionByZero::Error);
        }
    });

    let plan = percent_plan(&model);

    assert_eq!(
        named(&plan, "sales_to_budget_percent").derived,
        Some(formula("SALES_TO_BUDGET_PERCENT", DivisionByZero::Error))
    );
    let lower = plan.inputs()[0];
    assert_eq!(
        named(lower, "metric_1_input_1").derived,
        Some(formula("SALES_TO_BUDGET", DivisionByZero::Null))
    );
    assert_eq!(named(lower, "metric_1_component_1").derived, None);
}

#[test]
fn e6_4_6a_the_final_project_names_the_derived_metric_of_a_formula() {
    let plan = derived_plan(
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(0, 1)], true),
        ],
        vec![region()],
        &["SALES_TO_BUDGET"],
    );

    assert_eq!(
        named(&plan, "sales_to_budget").derived,
        Some(formula("SALES_TO_BUDGET", DivisionByZero::Null))
    );
    assert_eq!(named(&plan, "name_0").derived, None);
}

#[test]
fn der_3_1_a_derived_metric_without_a_rule_follows_e0f_6_1() {
    let model = changed_model("SALES_TO_BUDGET", |metric| {
        metric.derived = None;
    });

    let plan = plan_over(
        &model,
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(0, 1)], true),
        ],
        vec![region()],
        &["SALES_TO_BUDGET"],
    )
    .expect("a plan");

    assert_eq!(named(&plan, "sales_to_budget").derived, None);
}

#[test]
fn der_4_2_an_input_with_several_components_is_recomputed_in_a_lower_project() {
    let plan = derived_plan(
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(0, 1), (0, 2)], true),
        ],
        vec![region()],
        &["GAP_SHARE"],
    );

    let gap = binary(
        BinaryOperator::Subtract,
        column_reference("metric_1_component_1"),
        column_reference("metric_1_component_2"),
    );
    let lower = plan.inputs()[0];
    assert_eq!(named(lower, "metric_1_input_1").expression, gap);
    assert_eq!(named(lower, "metric_1_input_1").derived, None);
    assert_eq!(
        metric_expression(&plan, "gap_share"),
        ratio("metric_1_input_1", "metric_1_component_3")
    );
    let (_, _, columns) = stitch_parts(&plan);
    let names: Vec<_> = columns.iter().map(|column| column.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "metric_1_component_2",
            "metric_1_component_3",
            "metric_1_component_1"
        ]
    );
}

#[test]
fn der_4_1_a_base_metric_beside_a_derived_metric_keeps_its_own_components() {
    let plan = derived_plan(
        vec![branch("BUDGETS", &[(0, 0), (0, 1), (1, 0)], true)],
        vec![region()],
        &["AVERAGE_BUDGET", "BUDGET_TOTAL"],
    );

    assert_eq!(
        metric_expression(&plan, "budget_total"),
        column_reference("metric_2_component_1")
    );
    assert_eq!(
        plan.schema().columns()[1..],
        [
            column("average_budget", float(), true),
            column("budget_total", decimal(38, 2), true),
        ]
    );
}

fn violation(result: Result<PlanNode, PlanError>) -> InvariantViolation {
    match result {
        Err(PlanError::Invariant { violation, .. }) => violation,
        other => panic!("expected an internal error, got {other:?}"),
    }
}

#[test]
fn der_2_4_a_cycle_of_derived_metrics_is_an_internal_error() {
    let model = changed_model("SALES_TO_BUDGET", |metric| {
        metric.expression = Some(tree("AGG(sales_to_budget_percent) / AGG(budget_total)"));
    });

    let result = plan_over(
        &model,
        vec![branch("BUDGETS", &[(0, 0)], false)],
        Vec::new(),
        &["SALES_TO_BUDGET_PERCENT"],
    );

    assert_eq!(violation(result), InvariantViolation::DerivedCycle);
}

#[test]
fn e4_7_9_a_non_additive_input_plans_as_its_deduced_additivity() {
    let model = changed_model("BUDGET_TOTAL", |metric| {
        metric.declared_additivity = Some(DeclaredAdditivity::NonAdditive);
    });
    let plan = |model| {
        plan_over(
            model,
            vec![branch("BUDGETS", &[(0, 0), (0, 1)], false)],
            Vec::new(),
            &["AVERAGE_BUDGET"],
        )
    };

    assert_eq!(plan(&model), plan(&derived_model()));
}

#[test]
fn vs_1_a_context_metric_read_as_an_input_is_an_internal_error() {
    let model = changed_model("SALES_TO_BUDGET", |metric| {
        metric.context = Some(ContextMetric {
            base: name("SALES_TOTAL"),
            modifier: ContextModifier::PriorPeriod {
                time_dimension: FieldPath {
                    dataset: name("ORDERS"),
                    field: name("ORDER_DATE"),
                },
                grain: Grain::Year,
                offset: std::num::NonZeroU64::MIN,
            },
        });
    });

    assert_eq!(
        violation(percent_plan_result(&model)),
        InvariantViolation::ContextMetric
    );
}

fn percent_plan_result(model: &InternalModel) -> Result<PlanNode, PlanError> {
    plan_over(
        model,
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(0, 1)], true),
        ],
        vec![region()],
        &["SALES_TO_BUDGET_PERCENT"],
    )
}
