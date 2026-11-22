//! QRY-5.1, QRY-5.2, DEV-8 and E2-3.3: E2 binds a SQL-form query like a wide-table query, over
//! the one relation of its `FROM`.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_e2_bind::BindCode;
use menura_model::expression::{Expression, Identifier, Reference};
use menura_model::internal_model::FieldPath;
use menura_representations::bound_query::{
    AuthorizedObject, BoundDimensionKind, BoundMetricKind, MetricPath,
};
use menura_representations::columns::OutputColumn;

use support::model::{model, name};
use support::query::{column, equals_text, metric_aggregate};
use support::sql::{
    agg_item, expression_item, field_item, inline_item, relation, sql_columns, sql_where, upper,
    where_filter,
};
use support::{bind_query, first_error, run};

const ANALYST: &[&str] = &["analyst"];

fn field(dataset: &str, field: &str) -> FieldPath {
    FieldPath {
        dataset: name(dataset),
        field: name(field),
    }
}

/// The reference E2 writes for a bound field: `"DATASET"."FIELD"`, normalized.
#[cfg(test)]
fn bound_field(dataset: &str, field: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: Identifier::delimited(dataset),
        name: Identifier::delimited(field).expect("an identifier"),
    })
}

/// The `AGG(…)` E2 writes for a bound metric of the semantic model.
#[cfg(test)]
fn bound_model_metric(metric: &str) -> Expression {
    Expression::MetricAggregate(Reference {
        qualifier: None,
        name: Identifier::delimited(metric).expect("an identifier"),
    })
}

/// The `AGG(…)` E2 writes for a bound dataset metric.
#[cfg(test)]
fn bound_dataset_metric(dataset: &str, metric: &str) -> Expression {
    Expression::MetricAggregate(Reference {
        qualifier: Identifier::delimited(dataset),
        name: Identifier::delimited(metric).expect("an identifier"),
    })
}

#[test]
fn qry_5_1_each_dataset_is_the_relation_model_dot_dataset() {
    let tree = sql_columns(
        relation("sales", "orders", None, 30),
        vec![field_item("region", 8), agg_item("revenue", "r", 16)],
    );
    let authorized = run(&tree, ANALYST).expect("authorized");
    let columns: Vec<_> = authorized.columns().iter().collect();
    let [
        OutputColumn::Dimension(region),
        OutputColumn::Metric(revenue),
    ] = columns.as_slice()
    else {
        panic!("expected a dimension then a metric, got {columns:?}");
    };
    assert_eq!(
        region.kind,
        BoundDimensionKind::Field {
            field: field("ORDERS", "REGION"),
            time_grain: None,
            time_grain_origin: None,
        }
    );
    assert_eq!(
        revenue.kind,
        BoundMetricKind::Reference(MetricPath::SemanticModel(name("REVENUE")))
    );
}

#[test]
fn qry_5_1_the_relation_dataset_is_authorized_first() {
    let tree = sql_columns(
        relation("sales", "orders", None, 30),
        vec![agg_item("order_total", "t", 8)],
    );
    let authorized = run(&tree, ANALYST).expect("authorized");
    assert_eq!(
        authorized
            .authorization()
            .first()
            .map(|decision| &decision.object),
        Some(&AuthorizedObject::Dataset(name("ORDERS")))
    );
}

#[test]
fn qry_5_1_fields_may_be_qualified_by_the_alias_or_the_dataset_name() {
    let tree = sql_columns(
        relation("sales", "orders", Some("o"), 30),
        vec![
            field_item("o.region", 8),
            field_item("orders.customer_id", 18),
            agg_item("o.revenue", "r", 40),
        ],
    );
    let authorized = run(&tree, ANALYST).expect("authorized");
    let fields: Vec<_> = authorized
        .columns()
        .dimensions()
        .map(|dimension| dimension.kind.clone())
        .collect();
    assert_eq!(
        fields,
        vec![
            BoundDimensionKind::Field {
                field: field("ORDERS", "REGION"),
                time_grain: None,
                time_grain_origin: None,
            },
            BoundDimensionKind::Field {
                field: field("ORDERS", "CUSTOMER_ID"),
                time_grain: None,
                time_grain_origin: None,
            },
        ]
    );
}

#[test]
fn qry_5_2_a_dataset_metric_of_the_relation_is_exposed() {
    let tree = sql_columns(
        relation("sales", "orders", Some("o"), 30),
        vec![agg_item("o.order_total", "t", 8)],
    );
    let authorized = run(&tree, ANALYST).expect("authorized");
    let metrics: Vec<_> = authorized
        .columns()
        .metrics()
        .map(|metric| metric.kind.clone())
        .collect();
    assert_eq!(
        metrics,
        vec![BoundMetricKind::Reference(MetricPath::Dataset {
            dataset: name("ORDERS"),
            metric: name("ORDER_TOTAL"),
        })]
    );
}

#[test]
fn dev_8_a_model_metric_whose_components_are_all_on_the_relation_is_exposed() {
    let tree = sql_columns(
        relation("sales", "orders", Some("o"), 30),
        vec![agg_item("o.revenue", "r", 8), agg_item("revenue", "s", 20)],
    );
    let authorized = run(&tree, ANALYST).expect("authorized");
    let metrics: Vec<_> = authorized
        .columns()
        .metrics()
        .map(|metric| metric.kind.clone())
        .collect();
    let revenue = BoundMetricKind::Reference(MetricPath::SemanticModel(name("REVENUE")));
    assert_eq!(metrics, vec![revenue.clone(), revenue]);
}

#[test]
fn qry_5_2_a_model_metric_on_another_dataset_is_not_exposed() {
    let tree = sql_columns(
        relation("sales", "customers", None, 30),
        vec![field_item("region", 8), agg_item("revenue", "r", 16)],
    );
    assert_eq!(
        first_error(&tree, ANALYST),
        (BindCode::NameNotFound, "/sql".to_owned())
    );
}

#[test]
fn e2_3_3_an_expression_dimension_reads_fields_of_the_relation() {
    let tree = sql_columns(
        relation("sales", "orders", Some("o"), 40),
        vec![
            expression_item(upper(column("o.region")), "r", 8),
            agg_item("revenue", "n", 25),
        ],
    );
    let authorized = run(&tree, ANALYST).expect("authorized");
    let dimensions: Vec<_> = authorized
        .columns()
        .dimensions()
        .map(|dimension| dimension.kind.clone())
        .collect();
    assert_eq!(
        dimensions,
        vec![BoundDimensionKind::Expression {
            dataset: name("ORDERS"),
            expression: upper(bound_field("ORDERS", "REGION")),
        }]
    );
}

#[test]
fn e2_3_3_an_inline_metric_reads_metrics_exposed_on_the_relation() {
    let ratio = Expression::Binary {
        operator: menura_model::expression::BinaryOperator::Divide,
        left: Box::new(metric_aggregate("revenue")),
        right: Box::new(metric_aggregate("o.order_total")),
    };
    let tree = sql_columns(
        relation("sales", "orders", Some("o"), 60),
        vec![inline_item(ratio, "ratio", 8)],
    );
    let authorized = run(&tree, ANALYST).expect("authorized");
    let metrics: Vec<_> = authorized
        .columns()
        .metrics()
        .map(|metric| metric.kind.clone())
        .collect();
    assert_eq!(
        metrics,
        vec![BoundMetricKind::Inline(Expression::Binary {
            operator: menura_model::expression::BinaryOperator::Divide,
            left: Box::new(bound_model_metric("REVENUE")),
            right: Box::new(bound_dataset_metric("ORDERS", "ORDER_TOTAL")),
        })]
    );
}

#[test]
fn qry_5_1_where_is_the_query_filter_of_the_relation() {
    let tree = sql_where(
        relation("sales", "orders", Some("o"), 30),
        vec![agg_item("revenue", "r", 8)],
        where_filter(equals_text(column("region")), 50),
    );
    let authorized = run(&tree, ANALYST).expect("authorized");
    let [filter] = authorized.filters() else {
        panic!("expected one filter");
    };
    assert_eq!(filter.datasets, vec![name("ORDERS")]);
    assert_eq!(
        filter.expression,
        equals_text(bound_field("ORDERS", "REGION"))
    );
}

#[test]
fn qry_5_1_a_sql_query_binds_end_to_end() {
    let tree = sql_where(
        relation("sales", "orders", Some("o"), 40),
        vec![
            expression_item(upper(column("o.region")), "r", 8),
            agg_item("o.revenue", "n", 25),
        ],
        where_filter(equals_text(column("o.region")), 60),
    );
    let bound = bind_query(&tree, &model(), ANALYST).expect("bound");
    assert_eq!(bound.columns().iter().count(), 2);
}

#[test]
fn qry_5_1_a_where_that_is_not_boolean_keeps_its_filter_code() {
    let tree = sql_where(
        relation("sales", "orders", None, 30),
        vec![agg_item("revenue", "r", 8)],
        where_filter(column("region"), 50),
    );
    let diagnostic = support::bind_diagnostic(&tree, &model(), ANALYST);
    assert_eq!(diagnostic.code.as_str(), "E0F-06");
    assert_eq!(diagnostic.location, support::sql::sql_at(50));
}
