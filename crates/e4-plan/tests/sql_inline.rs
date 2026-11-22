//! The plan of an inline metric of the SQL form: an expression over `AGG(…)` terms, evaluated
//! per group after the terms (E1-4.2).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::{FloatBits, LogicalType};
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::logical_plan::{NamedExpression, Operation, PlanNode};
use menura_representations::origin::Origin;
use support::plan::{column_names, column_reference, kinds};
use support::sql::{SqlColumn, aggregated, expression_dimension, inline, select_item, sql_query};
use support::{anyone, branches, build_plan, model, tree};

fn plan(columns: Vec<SqlColumn>, component_counts: &[usize]) -> PlanNode {
    let query = sql_query(columns, BoundQueryParts::default());
    build_plan(
        &branches(component_counts),
        &query,
        &anyone(),
        &model(Vec::new()),
    )
    .expect("a plan")
}

fn projected(plan: &PlanNode) -> &[NamedExpression] {
    match plan.operation() {
        Operation::Project { expressions, .. } => expressions,
        other => panic!("not a Project: {other:?}"),
    }
}

#[test]
fn e1_4_2_an_inline_metric_is_evaluated_from_the_components_of_its_terms() {
    let columns = vec![
        expression_dimension(r#"LOWER("ORDERS"."REGION")"#, "region", 8),
        inline(
            r#"CAST(AGG("TOTAL_AMOUNT") AS DOUBLE) / AGG("ORDER_COUNT")"#,
            "per_order",
            20,
        ),
    ];

    let plan = plan(columns, &[2]);

    assert_eq!(
        kinds(&plan),
        ["Project", "Project", "Aggregate", "Project", "Scan"]
    );
    assert_eq!(plan.origin(), &Origin::Query(select_item(20)));
    assert_eq!(column_names(&plan), ["region", "per_order"]);
    let per_order = &projected(&plan)[1];
    assert_eq!(
        per_order.expression,
        tree(r#"CAST("metric_1_component_1" AS DOUBLE) / "metric_1_component_2""#)
    );
    assert_eq!(per_order.derived, None);
    let column = &plan.schema().columns()[1];
    assert_eq!(
        column.logical_type,
        LogicalType::Float {
            bits: FloatBits::SixtyFour
        }
    );
}

#[test]
fn e1_4_2_a_term_read_twice_is_computed_once() {
    let columns = vec![inline(
        r#"AGG("TOTAL_AMOUNT") + AGG("TOTAL_AMOUNT")"#,
        "twice",
        8,
    )];

    let plan = plan(columns, &[1]);

    assert_eq!(
        projected(&plan)[0].expression,
        tree(r#""metric_1_component_1" + "metric_1_component_1""#)
    );
}

#[test]
fn e1_4_2_model_metrics_are_computed_below_the_inline_formulas_from_the_model() {
    let columns = vec![
        inline(r#"AGG("ORDER_COUNT") + 1"#, "plus_one", 8),
        aggregated("PAID_AMOUNT", "paid", 30),
    ];

    let plan = plan(columns, &[1, 1]);

    let Operation::Project { input, .. } = plan.operation() else {
        panic!("a Project");
    };
    assert!(matches!(input.origin(), Origin::Model(_)));
    assert_eq!(
        column_names(input),
        ["metric_1_component_1", "metric_2_component_1", "metric_2"]
    );
    assert_eq!(column_names(&plan), ["plus_one", "paid"]);
}

#[test]
fn e1_4_2_a_term_of_several_components_is_an_input_column() {
    let columns = vec![
        aggregated("ORDER_COUNT", "orders", 8),
        inline(r#"AGG("AVERAGE_AMOUNT") * 2"#, "double_average", 30),
    ];

    let plan = plan(columns, &[1, 2]);

    assert_eq!(
        kinds(&plan),
        ["Project", "Project", "Project", "Aggregate", "Scan"]
    );
    assert_eq!(column_names(&plan), ["orders", "double_average"]);
    assert_eq!(projected(&plan)[0].expression, column_reference("metric_1"));
    assert_eq!(
        projected(&plan)[1].expression,
        tree(r#""metric_2_input_1" * 2"#)
    );
    let Operation::Project { input, .. } = plan.operation() else {
        panic!("a Project");
    };
    assert!(matches!(input.origin(), Origin::Model(_)));
}
