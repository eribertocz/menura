//! A development-line dataset metric that reads `source.column` (E0D-3): the `Scan` projects the
//! catalog column under a reserved name, and the component reads it there (E4-3, issue #264).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_e4_plan::{InvariantViolation, PlanError};
use menura_expressions::typing::big_integer;
use menura_model::expression::{AggregateCall, Expression};
use menura_representations::bound_query::{
    BoundMetric, BoundMetricKind, BoundQueryParts, MetricPath,
};
use menura_representations::logical_plan::{Operation, PlanNode};
use support::plan::{column, column_names, column_reference, node};
use support::{
    anyone, branches, build_plan, decimal_amount, field, metric, metric_reference, metrics,
    model_with, name, orders, query, tree,
};

/// The reserved prefix of a `Scan` column that names a catalog column.
const SOURCE_PREFIX: &str = "menura.source.";

/// The plan of the dataset metric `RAW_TOTAL`, written `expression`, of `home`.
fn plan_of(
    mut home: menura_model::internal_model::Dataset,
    expression: &str,
    components: usize,
) -> Result<PlanNode, PlanError> {
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
    build_plan(&branches(&[components]), &bound, &anyone(), &model)
}

/// The plan of the dataset metric `RAW_TOTAL` of `ORDERS`, written `expression`.
fn dataset_metric_plan(expression: &str, components: usize) -> PlanNode {
    plan_of(orders(Vec::new()), expression, components).expect("a plan")
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

#[test]
fn e4_3_a_source_column_has_its_catalog_type_and_nullability() {
    let plan = dataset_metric_plan("SUM(source.amount_cents) + SUM(source.\"Amount Cents\")", 2);
    assert_eq!(
        node(&plan, "Scan").schema().columns(),
        [
            column(
                &format!("{SOURCE_PREFIX}AMOUNT_CENTS"),
                big_integer(),
                false
            ),
            column(&format!("{SOURCE_PREFIX}Amount Cents"), big_integer(), true),
        ]
    );
}

#[test]
fn e4_3_a_source_column_read_twice_is_one_scan_column() {
    let plan = dataset_metric_plan("SUM(source.amount) + MAX(source.AMOUNT)", 2);
    assert_eq!(
        node(&plan, "Scan").schema().columns(),
        [column(
            &format!("{SOURCE_PREFIX}AMOUNT"),
            decimal_amount(),
            true
        )]
    );
}

#[test]
fn e4_3_the_prefix_grows_until_no_field_name_starts_with_it() {
    let mut home = orders(Vec::new());
    for taken in ["menura.source.AMOUNT", "menura.source..x"] {
        let mut taken_field = field("AMOUNT", big_integer(), true);
        taken_field.name = name(taken);
        home.fields.push(taken_field);
    }
    let plan = plan_of(home, "SUM(source.amount)", 1).expect("a plan");
    assert_eq!(
        column_names(node(&plan, "Scan")),
        vec!["menura.source...AMOUNT"]
    );
}

#[test]
fn e4_3_a_source_column_the_catalog_lacks_is_an_internal_error() {
    let result = plan_of(orders(Vec::new()), "SUM(source.discount)", 1);
    assert!(matches!(
        result,
        Err(PlanError::Invariant {
            violation: InvariantViolation::UnknownSourceColumn,
            ..
        })
    ));
}
