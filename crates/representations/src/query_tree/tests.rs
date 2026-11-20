//! Unit tests of the query tree (E1-5, E1-5.1).

use menura_contracts::types::common::Grain;
use menura_contracts::types::query::{DateBounds, Direction, NullsOrder, RelativeRange};
use menura_model::expression::Identifier;

use super::{
    QueryDateRange, QueryDimension, QueryDimensionKind, QueryFilter, QueryLimit, QueryMetric,
    QueryMetricKind, QueryOrderItem, QueryPath, QueryTree, QueryTreeParts, TimeGrainChoice,
};
use crate::columns::OutputColumn;
use crate::names::OutputName;
use crate::test_support::{at, in_sql, reference, true_literal};

fn dimension(field: &str, pointer: &str) -> QueryDimension {
    let field_reference = reference("orders", field);
    QueryDimension {
        output_name: OutputName::from_reference(&field_reference),
        kind: QueryDimensionKind::Field {
            reference: field_reference,
            time_grain: None,
        },
        origin: at(pointer),
    }
}

fn metric(name: &str, pointer: &str) -> QueryMetric {
    let metric_reference = reference("orders", name);
    QueryMetric {
        output_name: OutputName::from_reference(&metric_reference),
        kind: QueryMetricKind::Reference(metric_reference),
        origin: at(pointer),
    }
}

fn identifier(text: &str) -> Identifier {
    Identifier::regular(text).expect("a regular identifier")
}

fn every_element() -> QueryTreeParts {
    QueryTreeParts {
        filters: vec![QueryFilter {
            expression: true_literal(),
            origin: at("/query/filter"),
        }],
        date_range: Some(QueryDateRange {
            dimension: reference("orders", "ordered_at"),
            bounds: DateBounds::Relative(RelativeRange {
                unit: Grain::Month,
                count: 3,
                include_current: None,
            }),
            origin: at("/query/nonportable_extensions/0/data/date_range"),
        }),
        paths: vec![
            QueryPath::new(
                identifier("orders"),
                identifier("customers"),
                vec![identifier("orders_to_customers")],
                at("/query/nonportable_extensions/0/data/paths/0"),
            )
            .expect("one relationship"),
        ],
        order: vec![QueryOrderItem {
            output: OutputName::new("revenue"),
            direction: Direction::Desc,
            nulls: NullsOrder::Last,
            origin: at("/query/nonportable_extensions/0/data/order_by/0"),
        }],
        limit: Some(QueryLimit {
            rows: 10,
            origin: at("/query/nonportable_extensions/0/data/limit"),
        }),
    }
}

#[test]
fn e1_5_wide_table_form_puts_dimensions_before_metrics() {
    let tree = QueryTree::wide_table(
        vec![
            dimension("region", "/query/dimensions/0"),
            dimension("status", "/query/dimensions/1"),
        ],
        vec![metric("revenue", "/query/metrics/0")],
        QueryTreeParts::default(),
    );

    let names: Vec<&str> = tree
        .columns()
        .iter()
        .map(|column| match column {
            OutputColumn::Dimension(dimension) => dimension.output_name.as_str(),
            OutputColumn::Metric(metric) => metric.output_name.as_str(),
        })
        .collect();
    assert_eq!(names, ["region", "status", "revenue"]);
    assert!(matches!(
        tree.columns().get(2),
        Some(OutputColumn::Metric(_))
    ));
}

#[test]
fn e1_5_sql_form_keeps_select_order() {
    let mut first = metric("revenue", "/sql");
    first.origin = in_sql(1, 8);
    let mut second = dimension("region", "/sql");
    second.origin = in_sql(1, 25);
    let tree = QueryTree::in_select_order(
        vec![OutputColumn::Metric(first), OutputColumn::Dimension(second)],
        QueryTreeParts::default(),
    );

    assert!(matches!(
        tree.columns().first(),
        Some(OutputColumn::Metric(_))
    ));
    let dimensions: Vec<&QueryDimension> = tree.columns().dimensions().collect();
    assert_eq!(dimensions.len(), 1);
    assert_eq!(
        dimensions
            .first()
            .map(|dimension| dimension.origin.position()),
        Some(in_sql(1, 25).position())
    );
}

#[test]
fn e1_5_keeps_every_element_with_its_origin() {
    let mut grained = dimension("ordered_at", "/query/dimensions/0");
    grained.kind = QueryDimensionKind::Field {
        reference: reference("orders", "ordered_at"),
        time_grain: Some(TimeGrainChoice {
            grain: Grain::Month,
            origin: at("/query/nonportable_extensions/0/data/time_grains/0"),
        }),
    };
    let inline = QueryMetric {
        output_name: OutputName::new("double_revenue"),
        kind: QueryMetricKind::Inline(true_literal()),
        origin: at("/query/metrics/1"),
    };
    let tree = QueryTree::wide_table(
        vec![grained],
        vec![metric("revenue", "/query/metrics/0"), inline],
        every_element(),
    );

    assert_eq!(tree.columns().metrics().count(), 2);
    assert_eq!(tree.filters().len(), 1);
    assert_eq!(
        tree.date_range().map(|range| range.origin.pointer()),
        Some("/query/nonportable_extensions/0/data/date_range")
    );
    let path = tree.paths().first().expect("one path");
    assert_eq!(path.relationships().len(), 1);
    assert_eq!(
        path.origin().pointer(),
        "/query/nonportable_extensions/0/data/paths/0"
    );
    assert_eq!(tree.order().len(), 1);
    assert_eq!(tree.limit().map(|limit| limit.rows), Some(10));
}

#[test]
fn e1_5_a_path_names_at_least_one_relationship() {
    let path = QueryPath::new(
        identifier("orders"),
        identifier("customers"),
        Vec::new(),
        at("/query/nonportable_extensions/0/data/paths/0"),
    );
    assert!(path.is_none());
}

#[test]
fn e1_5_1_output_name_is_the_alias_or_the_last_part_of_the_reference() {
    assert_eq!(OutputName::new("Total Revenue").as_str(), "Total Revenue");
    assert_eq!(
        OutputName::from_reference(&reference("orders", "region")).as_str(),
        "region"
    );
}
