//! Stitching of several branches by the dimension keys (E4-3.7, E4-5, PLN-1).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::LogicalType;
use menura_expressions::typing::{big_integer, decimal};
use menura_model::expression::{BinaryOperator, Expression};
use menura_model::expression::{IntegerLiteral, Literal};
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::logical_plan::{Operation, PlanNode, StitchColumn};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use support::join_plan::scan_dataset;
use support::plan::{column, column_names, column_reference, kind, kinds};
use support::stitch::{branch, region, sales_and_budget_plan, stitch_parts, stitch_plan};
use support::{limit, located};

fn home_of(branch_plan: &PlanNode) -> &str {
    let mut current = branch_plan;
    while let Some(first) = current.inputs().first() {
        current = first;
    }
    scan_dataset(current)
}

fn stitched(name: &str, branch: usize, empty_value: Option<Literal>) -> StitchColumn {
    StitchColumn {
        name: ColumnName::new(name),
        branch,
        empty_value,
    }
}

fn zero() -> Literal {
    Literal::Integer(IntegerLiteral::new("0").expect("a literal"))
}

fn project_expressions(plan: &PlanNode) -> Vec<(&str, &Expression)> {
    match plan.operation() {
        Operation::Project { expressions, .. } => expressions
            .iter()
            .map(|named| (named.name.as_str(), &named.expression))
            .collect(),
        _ => panic!("not a Project"),
    }
}

#[test]
fn e4_3_7_two_branches_are_stitched_in_branch_order() {
    let plan = sales_and_budget_plan();

    assert_eq!(kinds(&plan)[..2], ["Project", "Stitch"]);
    let (branches, ..) = stitch_parts(&plan);
    let homes: Vec<_> = branches.iter().map(home_of).collect();
    assert_eq!(homes, ["BUDGETS", "ORDERS"]);
    assert!(branches.iter().all(|branch| kind(branch) == "Aggregate"));
    assert!(!plan.is_simple_plan());
}

#[test]
fn e4_5_0a_the_keys_are_the_dimension_columns_of_every_branch() {
    let plan = sales_and_budget_plan();
    let (branches, keys, ..) = stitch_parts(&plan);

    assert_eq!(keys, ["dimension_1"]);
    for branch in branches {
        assert_eq!(column_names(branch)[0], "dimension_1");
    }
}

#[test]
fn e4_5_0b_a_count_is_zero_and_a_sum_is_null_for_a_missing_branch_row() {
    let plan = sales_and_budget_plan();
    let (_, _, columns) = stitch_parts(&plan);

    assert_eq!(
        columns,
        [
            stitched("metric_2_component_1", 0, Some(zero())),
            stitched("metric_1_component_1", 1, None),
        ]
    );
}

/// The plan of `REGIONS.NAME`, `SALES_TOTAL` and one `BUDGETS` metric per aggregate.
fn every_aggregate_plan() -> PlanNode {
    let budget_metrics = [
        "BUDGET_COUNT",
        "BUDGET_ROWS",
        "BUDGET_DISTINCT",
        "BUDGET_MINIMUM",
        "BUDGET_MAXIMUM",
        "BUDGET_AVERAGE",
        "BUDGET_MEDIAN",
    ];
    let mut metric_names = vec!["SALES_TOTAL"];
    metric_names.extend(budget_metrics);
    let budget_components: Vec<_> = (1..=budget_metrics.len())
        .map(|metric| (metric, 0))
        .collect();
    stitch_plan(
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &budget_components, true),
        ],
        vec![region()],
        &metric_names,
        BoundQueryParts::default(),
    )
}

#[test]
fn e4_5_0b_every_count_is_zero_and_every_other_aggregate_is_null() {
    let plan = every_aggregate_plan();

    let (_, _, columns) = stitch_parts(&plan);
    let empty_values: Vec<_> = columns
        .iter()
        .map(|column| (column.name.as_str(), column.empty_value.clone()))
        .collect();
    assert_eq!(
        empty_values,
        [
            ("metric_2_component_1", Some(zero())),
            ("metric_3_component_1", Some(zero())),
            ("metric_4_component_1", Some(zero())),
            ("metric_5_component_1", None),
            ("metric_6_component_1", None),
            ("metric_7_component_1", None),
            ("metric_8_component_1", None),
            ("metric_1_component_1", None),
        ]
    );
    let nullable: Vec<_> = support::plan::node(&plan, "Stitch")
        .schema()
        .columns()
        .iter()
        .map(|column| column.nullable)
        .collect();
    assert_eq!(
        nullable,
        [true, false, false, false, true, true, true, true, true]
    );
}

#[test]
fn e4_5_1_each_column_names_the_branch_that_supplies_it() {
    let plan = sales_and_budget_plan();
    let (branches, _, columns) = stitch_parts(&plan);

    for column in columns {
        let branch = &branches[column.branch];
        assert!(column_names(branch).contains(&column.name.as_str()));
    }
}

#[test]
fn e4_3_11_pln_1_the_stitch_outputs_the_keys_then_the_components_in_branch_order() {
    let plan = sales_and_budget_plan();
    let stitch = support::plan::node(&plan, "Stitch");

    assert_eq!(
        stitch.schema().columns(),
        [
            column("dimension_1", LogicalType::Text, true),
            column("metric_2_component_1", big_integer(), false),
            column("metric_1_component_1", decimal(38, 2), true),
        ]
    );
    assert_eq!(
        stitch.origin(),
        &Origin::Model(located("/semantic_model/0/metrics/SALES_TOTAL"))
    );
}

#[test]
fn e4_5_0c_metrics_are_evaluated_from_the_stitched_components() {
    let plan = sales_and_budget_plan();

    assert_eq!(
        project_expressions(&plan),
        [
            ("name_0", &column_reference("dimension_1")),
            ("sales_total", &column_reference("metric_1_component_1")),
            ("budget_count", &column_reference("metric_2_component_1")),
        ]
    );
    assert_eq!(
        plan.schema().columns(),
        [
            column("name_0", LogicalType::Text, true),
            column("sales_total", decimal(38, 2), true),
            column("budget_count", big_integer(), false),
        ]
    );
}

#[test]
fn e4_5_0c_a_metric_whose_components_span_two_datasets_reads_both_branches() {
    let plan = stitch_plan(
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(0, 1)], true),
        ],
        vec![region()],
        &["GAP"],
        BoundQueryParts::default(),
    );

    let (_, _, columns) = stitch_parts(&plan);
    assert_eq!(
        columns,
        [
            stitched("metric_1_component_2", 0, None),
            stitched("metric_1_component_1", 1, None),
        ]
    );
    let gap = Expression::Binary {
        operator: BinaryOperator::Subtract,
        left: Box::new(column_reference("metric_1_component_1")),
        right: Box::new(column_reference("metric_1_component_2")),
    };
    assert_eq!(project_expressions(&plan)[1], ("gap", &gap));
}

#[test]
fn e4_5_2_without_dimensions_the_stitch_has_no_keys() {
    let plan = stitch_plan(
        vec![
            branch("ORDERS", &[(0, 0)], false),
            branch("BUDGETS", &[(1, 0)], false),
        ],
        Vec::new(),
        &["SALES_TOTAL", "BUDGET_COUNT"],
        BoundQueryParts::default(),
    );

    let (branches, keys, ..) = stitch_parts(&plan);
    assert!(keys.is_empty());
    for branch in branches {
        match branch.operation() {
            Operation::Aggregate { keys, .. } => assert!(keys.is_empty()),
            _ => panic!("not an Aggregate"),
        }
    }
}

#[test]
fn e4_9_sort_and_limit_apply_after_the_stitch() {
    let plan = stitch_plan(
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(1, 0)], true),
        ],
        vec![region()],
        &["SALES_TOTAL", "BUDGET_COUNT"],
        BoundQueryParts {
            limit: Some(limit(5)),
            ..BoundQueryParts::default()
        },
    );

    assert_eq!(kinds(&plan)[..3], ["Limit", "Project", "Stitch"]);
}
