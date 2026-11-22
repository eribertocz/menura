//! Golden SQL of a plan of the SQL form (E1-4, issue #356): the literals written in the SQL
//! text, in a `CASE` dimension and in an inline metric, are bound parameters numbered in order
//! of first appearance (E6-4.2), and the constants of the model metrics stay literals (E6-4.11).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use menura_representations::logical_plan::{PlanNode, SchemaColumn};
use menura_representations::origin::{Origin, QueryLocation, TextPosition};
use serde_json::json;
use support::{
    aggregate, assert_golden, big_integer, column, component, decimal, filter, from_model,
    generate, named, project, query, scan,
};

/// The origin of the `SELECT` item or clause at `column` of the SQL text.
fn sql_at(column: usize) -> Origin {
    Origin::Query(QueryLocation::new("/sql", TextPosition::new(1, column)))
}

/// The columns of `input`, each passed through.
fn pass_through(
    input: &PlanNode,
) -> (
    Vec<menura_representations::logical_plan::NamedExpression>,
    Vec<SchemaColumn>,
) {
    let columns = input.schema().columns().to_vec();
    let expressions = columns
        .iter()
        .map(|column| {
            named(
                column.name.as_str(),
                &format!("\"{}\"", column.name.as_str()),
            )
        })
        .collect();
    (expressions, columns)
}

/// `SELECT CASE WHEN region = 'x' THEN 'y' ELSE region END AS r,
/// COALESCE(AGG(total_amount), 0) AS padded, AGG(scaled) AS scaled FROM shop.orders
/// WHERE quantity > 1 GROUP BY ALL`, as E4 plans it: the `CASE` dimension is computed in a
/// `Project` from the query, the model metric `scaled` (`SUM(amount) * 100`) in a `Project`
/// from the model, and the inline formula in a `Project` from the query.
fn sql_form_plan() -> PlanNode {
    let filtered = filter(
        scan("ORDERS", &["AMOUNT", "QUANTITY", "REGION"]),
        r#""QUANTITY" > 1"#,
        sql_at(120),
    );
    let (mut expressions, mut columns) = pass_through(&filtered);
    expressions.push(named(
        "menura.dimension_1",
        r#"CASE WHEN "REGION" = 'x' THEN 'y' ELSE "REGION" END"#,
    ));
    columns.push(column("menura.dimension_1", LogicalType::Text, true));
    let dimensions = project(filtered, expressions, columns, sql_at(8));
    let aggregated = aggregate(
        dimensions,
        vec![named("dimension_1", r#""menura.dimension_1""#)],
        vec![
            component("metric_1_component_1", r#"SUM("AMOUNT")"#, None),
            component("metric_2_component_1", r#"SUM("AMOUNT")"#, None),
        ],
        vec![
            column("dimension_1", LogicalType::Text, true),
            column("metric_1_component_1", decimal(38, 2), true),
            column("metric_2_component_1", decimal(38, 2), true),
        ],
    );
    let (mut expressions, mut columns) = pass_through(&aggregated);
    expressions.push(named("metric_2", r#""metric_2_component_1" * 100"#));
    columns.push(column("metric_2", decimal(38, 2), true));
    let model_metrics = project(
        aggregated,
        expressions,
        columns,
        from_model("/semantic_model/0/metrics/1"),
    );
    project(
        model_metrics,
        vec![
            named("r", r#""dimension_1""#),
            named("padded", r#"COALESCE("metric_1_component_1", 0)"#),
            named("scaled", r#""metric_2""#),
        ],
        vec![
            column("r", LogicalType::Text, true),
            column("padded", decimal(38, 2), false),
            column("scaled", decimal(38, 2), true),
        ],
        sql_at(60),
    )
}

#[test]
fn e6_4_2_literals_of_the_sql_text_are_parameters_in_order_of_first_appearance() {
    let generated = generate(&sql_form_plan(), &query(&["r"], &["padded", "scaled"])).expect("SQL");

    assert_golden(&generated.sql, include_str!("golden/sql_form_literals.sql"));
    let values: Vec<_> = generated
        .parameters
        .iter()
        .map(|parameter| (parameter.position, parameter.value.clone()))
        .collect();
    assert_eq!(
        values,
        [
            (1, json!(1)),
            (2, json!("x")),
            (3, json!("y")),
            (4, json!(0)),
        ]
    );
    assert_eq!(generated.parameters[0].logical_type, big_integer());
}

/// `SELECT CASE WHEN quantity > 1 THEN 'big' ELSE 'small' END AS size, AGG(total_amount) AS
/// amount FROM shop.orders WHERE quantity > 1 GROUP BY ALL`: the two `1` are written at two
/// places of the SQL text, in two nodes, each the first literal of its node.
fn equal_literals_plan() -> PlanNode {
    let filtered = filter(
        scan("ORDERS", &["AMOUNT", "QUANTITY"]),
        r#""QUANTITY" > 1"#,
        sql_at(100),
    );
    let (mut expressions, mut columns) = pass_through(&filtered);
    expressions.push(named(
        "menura.dimension_1",
        r#"CASE WHEN "QUANTITY" > 1 THEN 'big' ELSE 'small' END"#,
    ));
    columns.push(column("menura.dimension_1", LogicalType::Text, false));
    let dimensions = project(filtered, expressions, columns, sql_at(8));
    let aggregated = aggregate(
        dimensions,
        vec![named("dimension_1", r#""menura.dimension_1""#)],
        vec![component("metric_1_component_1", r#"SUM("AMOUNT")"#, None)],
        vec![
            column("dimension_1", LogicalType::Text, false),
            column("metric_1_component_1", decimal(38, 2), true),
        ],
    );
    project(
        aggregated,
        vec![
            named("size", r#""dimension_1""#),
            named("amount", r#""metric_1_component_1""#),
        ],
        vec![
            column("size", LogicalType::Text, false),
            column("amount", decimal(38, 2), true),
        ],
        from_model("/semantic_model/0/metrics/0"),
    )
}

#[test]
fn e6_4_2_equal_literals_at_two_places_of_the_sql_text_are_two_parameters() {
    let generated = generate(&equal_literals_plan(), &query(&["size"], &["amount"])).expect("SQL");

    assert_golden(
        &generated.sql,
        include_str!("golden/sql_form_equal_literals.sql"),
    );
    let values: Vec<_> = generated
        .parameters
        .iter()
        .map(|parameter| (parameter.position, parameter.value.clone()))
        .collect();
    assert_eq!(
        values,
        [
            (1, json!(1)),
            (2, json!(1)),
            (3, json!("big")),
            (4, json!("small")),
        ]
    );
}
