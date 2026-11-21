//! A model with two facts, `ORDERS` and `BUDGETS`, that share the dimension `REGIONS`, and
//! the branches of their queries, for the stitching tests (E4-5).

use menura_contracts::types::common::LogicalType;
use menura_expressions::typing::{big_integer, decimal};
use menura_model::expression::Expression;
use menura_model::internal_model::{
    Additivity, Component, Dataset, DecomposabilityClass, Descriptions, InternalModel, Metric,
    MetricClassification, Relationship, TypedValue, ValueType,
};
use menura_representations::bound_query::{BoundDimension, BoundMetric, BoundQueryParts};
use menura_representations::logical_plan::{Operation, PlanNode, StitchColumn};
use menura_representations::names::ColumnName;
use menura_representations::paths::{
    Branch, Branches, ComponentKey, ElementPath, JoinStep, JoinTree, QueryElement,
};

use super::joins::joined_dimension;
use super::plan::node;
use super::{
    anyone, build_plan, field, located, metric_reference, model_with, name, orders, query, tree,
};

fn fact(dataset_name: &str, index: usize) -> Dataset {
    let mut dataset = orders(Vec::new());
    dataset.name = name(dataset_name);
    dataset.source = format!("main.{}", dataset_name.to_lowercase());
    dataset.fields = vec![
        field("AMOUNT", decimal(12, 2), true),
        field("REGION_ID", big_integer(), false),
    ];
    dataset.location = located(&format!("/semantic_model/0/datasets/{index}"));
    dataset
}

fn regions() -> Dataset {
    let mut regions = fact("REGIONS", 2);
    regions.fields = vec![
        field("NAME", LogicalType::Text, true),
        field("REGION_ID", big_integer(), false),
    ];
    regions
}

fn to_regions(from: &str, index: usize) -> Relationship {
    Relationship {
        name: name(&format!("{from}_TO_REGIONS")),
        from: name(from),
        to: name("REGIONS"),
        from_columns: vec![name("REGION_ID")],
        to_columns: vec![name("REGION_ID")],
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: located(&format!("/semantic_model/0/relationships/{index}")),
    }
}

/// A metric of the semantic model whose aggregates, in written order, have the home datasets
/// `homes`.
fn metric(
    metric_name: &str,
    expression: &str,
    homes: &[&str],
    logical_type: LogicalType,
) -> Metric {
    let expression = tree(expression);
    let mut calls = Vec::new();
    collect_aggregates(&expression, &mut calls);
    let components = calls
        .into_iter()
        .zip(homes)
        .map(|(aggregate, home)| match aggregate {
            Expression::Aggregate { call, filter } => Component {
                call,
                filter: filter.map(|filter| *filter),
                home_dataset: name(home),
                class: DecomposabilityClass::Distributive,
            },
            _ => panic!("an aggregate"),
        })
        .collect();
    let nullable = !expression_is_count(&expression);
    let mut home_datasets: Vec<_> = homes.iter().map(|home| name(home)).collect();
    home_datasets.sort();
    home_datasets.dedup();
    Metric {
        name: name(metric_name),
        expression: Some(expression),
        typed: Some(TypedValue {
            value_type: ValueType::Logical(logical_type),
            nullable,
        }),
        classification: Some(MetricClassification {
            components,
            inputs: Vec::new(),
            home_datasets,
            additivity: Additivity::Additive,
            recomposable: true,
        }),
        declared_additivity: None,
        derived: None,
        context: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: located(&format!("/semantic_model/0/metrics/{metric_name}")),
    }
}

fn expression_is_count(expression: &Expression) -> bool {
    matches!(expression, Expression::Aggregate { call, .. }
        if matches!(call, menura_model::expression::AggregateCall::CountRows))
}

fn collect_aggregates(expression: &Expression, aggregates: &mut Vec<Expression>) {
    match expression {
        Expression::Aggregate { .. } => aggregates.push(expression.clone()),
        Expression::Binary { left, right, .. } => {
            collect_aggregates(left, aggregates);
            collect_aggregates(right, aggregates);
        }
        _ => {}
    }
}

/// The model of `BUDGETS` and `ORDERS`, each related to `REGIONS`, with the metrics
/// `BUDGET_COUNT` (`COUNT(*)` of `BUDGETS`), `BUDGET_TOTAL` (`SUM` of `BUDGETS`), `GAP`
/// (`ORDERS` minus `BUDGETS`) and `SALES_TOTAL` (`SUM` of `ORDERS`).
pub fn two_facts_model() -> InternalModel {
    let base = model_with(
        vec![fact("BUDGETS", 0), fact("ORDERS", 1), regions()],
        vec![
            metric("BUDGET_COUNT", "COUNT(*)", &["BUDGETS"], big_integer()),
            metric(
                "BUDGET_TOTAL",
                "SUM(budgets.amount)",
                &["BUDGETS"],
                decimal(38, 2),
            ),
            metric(
                "GAP",
                "SUM(orders.amount) - SUM(budgets.amount)",
                &["ORDERS", "BUDGETS"],
                decimal(38, 2),
            ),
            metric(
                "SALES_TOTAL",
                "SUM(orders.amount)",
                &["ORDERS"],
                decimal(38, 2),
            ),
        ],
    );
    let mut parts = base.parts().clone();
    parts.relationships = vec![to_regions("BUDGETS", 0), to_regions("ORDERS", 1)];
    InternalModel::new(parts, base.hash().clone())
}

fn region_tree(home: &str, with_dimension: bool) -> JoinTree {
    if !with_dimension {
        return JoinTree::default();
    }
    JoinTree::from_paths(vec![ElementPath {
        element: QueryElement::Dimension(0),
        steps: vec![JoinStep {
            relationship: name(&format!("{home}_TO_REGIONS")),
            from: name(home),
            to: name("REGIONS"),
        }],
    }])
}

/// The branch on `home` with the components `components` (metric, component), joined to
/// `REGIONS` when the query has the region dimension.
pub fn branch(home: &str, components: &[(usize, usize)], with_dimension: bool) -> Branch {
    Branch {
        home: name(home),
        components: components
            .iter()
            .map(|(metric, component)| ComponentKey {
                metric: *metric,
                component: *component,
            })
            .collect(),
        join_tree: region_tree(home, with_dimension),
    }
}

/// The dimension `REGIONS.NAME`.
pub fn region() -> BoundDimension {
    joined_dimension("REGIONS", "NAME", None, 0)
}

/// The plan over [`two_facts_model`] of `dimensions`, the metrics `metric_names` and
/// `branches`.
pub fn stitch_plan(
    branches: Vec<Branch>,
    dimensions: Vec<BoundDimension>,
    metric_names: &[&str],
    parts: BoundQueryParts,
) -> PlanNode {
    let metrics: Vec<BoundMetric> = metric_names
        .iter()
        .enumerate()
        .map(|(index, metric_name)| metric_reference(metric_name, index))
        .collect();
    build_plan(
        &Branches::new(branches).expect("distinct homes"),
        &query(dimensions, metrics, parts),
        &anyone(),
        &two_facts_model(),
    )
    .expect("a plan")
}

/// The plan of `REGIONS.NAME`, `SALES_TOTAL` and `BUDGET_COUNT`.
pub fn sales_and_budget_plan() -> PlanNode {
    stitch_plan(
        vec![
            branch("ORDERS", &[(0, 0)], true),
            branch("BUDGETS", &[(1, 0)], true),
        ],
        vec![region()],
        &["SALES_TOTAL", "BUDGET_COUNT"],
        BoundQueryParts::default(),
    )
}

/// The branches, keys and component columns of the `Stitch` of `plan`.
pub fn stitch_parts(plan: &PlanNode) -> (&[PlanNode], Vec<&str>, &[StitchColumn]) {
    match node(plan, "Stitch").operation() {
        Operation::Stitch {
            branches,
            keys,
            columns,
        } => (
            branches,
            keys.iter().map(ColumnName::as_str).collect(),
            columns,
        ),
        _ => panic!("not a Stitch"),
    }
}
