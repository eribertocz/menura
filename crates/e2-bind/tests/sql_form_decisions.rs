//! QRY-5.1, QRY-5.2, DEV-8 and E2-4.4 in the SQL form: how a name read through the `FROM`
//! relation resolves, and P5, the SQL form binds as the equivalent wide-table query.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_e2_bind::BindCode;
use menura_model::expression::{BinaryOperator, Expression, Identifier, Literal, Reference};
use menura_model::internal_model::{FieldPath, InternalModel};
use menura_representations::bound_query::{
    BoundDimensionKind, BoundMetricKind, BoundQuery, MetricPath,
};
use menura_representations::query_tree::{QueryRelation, QueryTree, QueryTreeParts};

use support::model::{add_semantic_metric, mark_metric, model, model_with, name};
use support::query::{column, dimension, equals_text, filter, metric, tree};
use support::sql::{agg_item, field_item, relation, sql_at, sql_columns, sql_where, where_filter};
use support::{bind_diagnostic, bind_query};

const ANALYST: &[&str] = &["analyst"];

/// The model with the metrics of the semantic model `REGION` and `UNIT_COST`, which share
/// their names with a field of `ORDERS` (`UNIT_COST` is restricted to admin), and
/// `ORDER_TOTAL`, which shares its name with a dataset metric of `ORDERS`. Each reads
/// `orders.amount` only.
#[cfg(test)]
fn shadowing_model() -> InternalModel {
    model_with(|parts| {
        for metric_name in ["REGION", "UNIT_COST", "ORDER_TOTAL"] {
            add_semantic_metric(parts, metric_name, "ORDERS", "AMOUNT");
        }
    })
}

#[cfg(test)]
fn bound(tree: &QueryTree, model: &InternalModel) -> BoundQuery {
    bind_query(tree, model, ANALYST).expect("bound")
}

#[cfg(test)]
fn first_code(tree: &QueryTree, model: &InternalModel) -> BindCode {
    bind_diagnostic(tree, model, ANALYST).code
}

fn metric_kinds(query: &BoundQuery) -> Vec<BoundMetricKind> {
    query
        .columns()
        .metrics()
        .map(|metric| metric.kind.clone())
        .collect()
}

fn dimension_kinds(query: &BoundQuery) -> Vec<BoundDimensionKind> {
    query
        .columns()
        .dimensions()
        .map(|dimension| dimension.kind.clone())
        .collect()
}

#[test]
fn qry_5_2_an_authorized_field_wins_over_a_same_named_metric() {
    let model = shadowing_model();
    let as_item = sql_columns(
        relation("sales", "orders", None, 40),
        vec![field_item("region", 8), agg_item("revenue", "r", 16)],
    );
    assert_eq!(
        dimension_kinds(&bound(&as_item, &model)),
        vec![BoundDimensionKind::Field {
            field: FieldPath {
                dataset: name("ORDERS"),
                field: name("REGION"),
            },
            time_grain: None,
            time_grain_origin: None,
        }]
    );
    let in_agg = sql_columns(
        relation("sales", "orders", None, 40),
        vec![agg_item("region", "r", 8)],
    );
    assert_eq!(first_code(&in_agg, &model), BindCode::FieldInMetrics);
}

#[test]
fn e2_4_4_an_unauthorized_field_does_not_hide_a_same_named_metric() {
    let model = shadowing_model();
    let in_agg = sql_columns(
        relation("sales", "orders", None, 40),
        vec![agg_item("unit_cost", "u", 8)],
    );
    assert_eq!(
        metric_kinds(&bound(&in_agg, &model)),
        vec![BoundMetricKind::Reference(MetricPath::SemanticModel(name(
            "UNIT_COST"
        )))]
    );
    let as_item = sql_columns(
        relation("sales", "orders", None, 40),
        vec![field_item("unit_cost", 8), agg_item("revenue", "r", 20)],
    );
    assert_eq!(first_code(&as_item, &model), BindCode::MetricInDimensions);
}

#[test]
fn qry_5_2_a_dataset_metric_is_looked_up_before_a_model_metric() {
    let tree = sql_columns(
        relation("sales", "orders", None, 40),
        vec![agg_item("order_total", "t", 8)],
    );
    assert_eq!(
        metric_kinds(&bound(&tree, &shadowing_model())),
        vec![BoundMetricKind::Reference(MetricPath::Dataset {
            dataset: name("ORDERS"),
            metric: name("ORDER_TOTAL"),
        })]
    );
}

#[cfg(test)]
fn relation_of_model(model: Identifier) -> QueryRelation {
    QueryRelation {
        model,
        dataset: Identifier::regular("orders").expect("an identifier"),
        alias: None,
        origin: sql_at(30),
    }
}

#[test]
fn qry_5_1_the_model_name_compares_under_e0d_1() {
    let matching = [
        Identifier::regular("SALES").expect("an identifier"),
        Identifier::regular("Sales").expect("an identifier"),
        Identifier::delimited("SALES").expect("an identifier"),
    ];
    for model_name in matching {
        let tree = sql_columns(
            relation_of_model(model_name),
            vec![agg_item("revenue", "r", 8)],
        );
        assert!(bind_query(&tree, &model(), ANALYST).is_ok());
    }
    let delimited_lower_case = sql_columns(
        relation_of_model(Identifier::delimited("sales").expect("an identifier")),
        vec![agg_item("revenue", "r", 8)],
    );
    assert_eq!(
        first_code(&delimited_lower_case, &model()),
        BindCode::NameNotFound
    );
}

#[test]
fn e2_4_4_an_unauthorized_field_in_where_reads_as_a_missing_one() {
    let with = |field: &str| {
        sql_where(
            relation("sales", "orders", None, 30),
            vec![agg_item("revenue", "r", 8)],
            where_filter(equals_text(column(field)), 50),
        )
    };
    let denied = bind_diagnostic(&with("unit_cost"), &model(), ANALYST);
    let missing = bind_diagnostic(&with("unit_cosx"), &model(), ANALYST);
    assert_eq!(denied.code, BindCode::NameNotFound);
    assert_eq!(denied.location, missing.location);
    assert_eq!(
        denied.message.replace("unit_cost", "unit_cosx"),
        missing.message
    );
}

#[test]
fn e2_4_4_an_unauthorized_metric_in_agg_reads_as_a_missing_one() {
    let with = |metric: &str| {
        sql_columns(
            relation("sales", "orders", None, 30),
            vec![agg_item(metric, "c", 8)],
        )
    };
    let denied = bind_diagnostic(&with("cost"), &model(), ANALYST);
    let missing = bind_diagnostic(&with("cosx"), &model(), ANALYST);
    assert_eq!(denied.code, BindCode::NameNotFound);
    assert_eq!(denied.message.replace("cost", "cosx"), missing.message);
}

#[test]
fn dev_8_an_unusable_model_metric_keeps_its_row_12a_code() {
    let model = model_with(|parts| {
        mark_metric(parts, "REVENUE", &["E0D-03"]);
        support::model::semantic_metric(parts, "REVENUE").classification = None;
    });
    let sql = sql_columns(
        relation("sales", "orders", None, 30),
        vec![agg_item("revenue", "r", 8)],
    );
    let wide = tree(
        Vec::new(),
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    assert_eq!(first_code(&sql, &model).as_str(), "E0D-03");
    assert_eq!(first_code(&wide, &model).as_str(), "E0D-03");
}

/// `"ORDERS"."REGION" = 'x'`: the wide-table filter that reads what `region = 'x'` reads
/// through the relation.
#[cfg(test)]
fn wide_table_region_filter() -> Expression {
    Expression::Binary {
        operator: BinaryOperator::Equal,
        left: Box::new(Expression::Reference(Reference {
            qualifier: Identifier::delimited("ORDERS"),
            name: Identifier::delimited("REGION").expect("an identifier"),
        })),
        right: Box::new(Expression::Literal(Literal::Text("x".to_owned()))),
    }
}

/// Everything of a bound query but where its elements are written.
#[cfg(test)]
fn without_origins(query: &BoundQuery) -> impl PartialEq + std::fmt::Debug + use<> {
    let columns: Vec<_> = query
        .columns()
        .iter()
        .map(|column| match column {
            menura_representations::columns::OutputColumn::Dimension(dimension) => (
                format!("{:?}", dimension.kind),
                dimension.output_name.clone(),
            ),
            menura_representations::columns::OutputColumn::Metric(metric) => {
                (format!("{:?}", metric.kind), metric.output_name.clone())
            }
        })
        .collect();
    let filters: Vec<_> = query
        .filters()
        .iter()
        .map(|filter| (filter.expression.clone(), filter.datasets.clone()))
        .collect();
    (
        columns,
        filters,
        query.authorization().to_vec(),
        query.subgraph().clone(),
        query.order().len(),
        query.limit().map(|limit| limit.rows),
    )
}

#[test]
fn qry_5_1_a_sql_query_binds_as_its_wide_table_equivalent() {
    let sql = sql_where(
        relation("sales", "orders", None, 40),
        vec![field_item("region", 8), agg_item("revenue", "revenue", 16)],
        where_filter(equals_text(column("region")), 60),
    );
    let wide = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts {
            filters: vec![filter("/query/filter", wide_table_region_filter())],
            ..QueryTreeParts::default()
        },
    );
    assert_eq!(
        without_origins(&bound(&sql, &model())),
        without_origins(&bound(&wide, &model()))
    );
}

#[test]
fn qry_5_1_a_sql_query_fails_as_its_wide_table_equivalent() {
    let pairs = [
        (
            vec![field_item("revenue", 8), agg_item("order_total", "t", 20)],
            tree(
                vec![dimension(0, "revenue")],
                vec![metric(0, "orders.order_total")],
                QueryTreeParts::default(),
            ),
        ),
        (
            vec![field_item("region", 8), agg_item("amount", "a", 20)],
            tree(
                vec![dimension(0, "orders.region")],
                vec![metric(0, "orders.amount")],
                QueryTreeParts::default(),
            ),
        ),
        (
            vec![field_item("nowhere", 8), agg_item("revenue", "r", 20)],
            tree(
                vec![dimension(0, "orders.nowhere")],
                vec![metric(0, "revenue")],
                QueryTreeParts::default(),
            ),
        ),
    ];
    for (columns, wide) in pairs {
        let sql = sql_columns(relation("sales", "orders", None, 40), columns);
        assert_eq!(first_code(&sql, &model()), first_code(&wide, &model()));
    }
}
