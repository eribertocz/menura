//! A development-line dataset metric that reads `source.column` (E0D-3): the `Scan` projects the
//! catalog column under a reserved name, and the component reads it there (E4-3, issue #264).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_e4_plan::build_plan;
use menura_expressions::typing::big_integer;
use menura_model::expression::{AggregateCall, Expression};
use menura_representations::bound_query::{
    BoundMetric, BoundMetricKind, BoundQueryParts, MetricPath,
};
use menura_representations::logical_plan::{Operation, PlanNode};
use support::plan::{column_names, column_reference, node};
use support::{
    anyone, branches, metric, metric_reference, metrics, model_with, name, orders, query, tree,
};

/// The reserved prefix of a `Scan` column that names a catalog column.
const SOURCE_PREFIX: &str = "menura.source.";

/// The plan of the dataset metric `RAW_TOTAL` of `ORDERS`, written `expression`.
fn dataset_metric_plan(expression: &str, components: usize) -> PlanNode {
    let mut home = orders(Vec::new());
    home.metrics = vec![metric("RAW_TOTAL", expression, big_integer())];
    let model = model_with(vec![home], metrics());
    let raw = BoundMetric {
        kind: BoundMetricKind::Reference(MetricPath::Dataset {
            dataset: name("ORDERS"),
            metric: name("RAW_TOTAL"),
        }),
        ..metric_reference("RAW_TOTAL", 0)
    };
    let bound = query(Vec::new(), vec![raw], BoundQueryParts::default());
    build_plan(&branches(&[components]), &bound, &anyone(), &model).expect("a plan")
}

/// The argument of the aggregate call of every component of the `Aggregate` of `plan`.
fn component_calls(plan: &PlanNode) -> Vec<AggregateCall> {
    match node(plan, "Aggregate").operation() {
        Operation::Aggregate { components, .. } => components
            .iter()
            .map(|component| component.call.clone())
            .collect(),
        _ => panic!("not an Aggregate"),
    }
}

/// The call `SUM` of the input column `column`.
fn sum_of(column: &str) -> AggregateCall {
    let Expression::Aggregate {
        call: AggregateCall::Simple { function, .. },
        ..
    } = tree("SUM(x)")
    else {
        panic!("not a simple aggregate");
    };
    AggregateCall::Simple {
        function,
        argument: Box::new(column_reference(column)),
    }
}

#[test]
fn e4_3_a_scan_projects_the_source_column_a_dataset_metric_reads() {
    let plan = dataset_metric_plan("SUM(source.amount_cents)", 1);
    let reserved = format!("{SOURCE_PREFIX}AMOUNT_CENTS");
    assert_eq!(column_names(node(&plan, "Scan")), vec![reserved.as_str()]);
}

#[test]
fn e0d_3_the_component_references_the_source_column_as_an_input_column() {
    let plan = dataset_metric_plan("SUM(source.amount_cents)", 1);
    let reserved = format!("{SOURCE_PREFIX}AMOUNT_CENTS");
    assert_eq!(component_calls(&plan), vec![sum_of(&reserved)]);
}

#[test]
fn e0d_1_5_a_delimited_source_column_keeps_its_exact_spelling() {
    let plan = dataset_metric_plan("SUM(source.\"Amount Cents\")", 1);
    let reserved = format!("{SOURCE_PREFIX}Amount Cents");
    assert_eq!(column_names(node(&plan, "Scan")), vec![reserved.as_str()]);
}

#[test]
fn e4_3_a_field_and_a_source_column_of_the_same_name_are_two_scan_columns() {
    let plan = dataset_metric_plan("SUM(amount) + SUM(source.amount)", 2);
    let reserved = format!("{SOURCE_PREFIX}AMOUNT");
    assert_eq!(
        column_names(node(&plan, "Scan")),
        vec!["AMOUNT", reserved.as_str()]
    );
    assert_eq!(
        component_calls(&plan),
        vec![sum_of("AMOUNT"), sum_of(&reserved)]
    );
}
