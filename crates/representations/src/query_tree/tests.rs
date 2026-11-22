//! Unit tests of the query tree (E1-5, E1-5.1).

use menura_contracts::types::common::Grain;
use menura_contracts::types::query::{DateBounds, Direction, NullsOrder, RelativeRange};

use super::{
    QueryDateRange, QueryDimension, QueryDimensionKind, QueryFilter, QueryLimit, QueryMetric,
    QueryMetricKind, QueryOrderItem, QueryPath, QueryReference, QueryRelation, QueryTree,
    QueryTreeParts, ReferencePart, TimeGrainChoice,
};
use menura_model::expression::Identifier;

use crate::columns::OutputColumn;
use crate::names::OutputName;
use crate::test_support::{at, in_sql, madrid_time, query_reference, true_literal};

fn dimension(field: &str, pointer: &str) -> QueryDimension {
    let field_reference = query_reference(&["orders", field]);
    QueryDimension {
        output_name: OutputName::from_reference(&field_reference),
        kind: QueryDimensionKind::Field {
            reference: field_reference,
        },
        origin: at(pointer),
    }
}

fn metric(name: &str, pointer: &str) -> QueryMetric {
    let metric_reference = query_reference(&["orders", name]);
    QueryMetric {
        output_name: OutputName::from_reference(&metric_reference),
        kind: QueryMetricKind::Reference(metric_reference),
        origin: at(pointer),
    }
}

fn every_element() -> QueryTreeParts {
    QueryTreeParts {
        filters: vec![QueryFilter {
            expression: true_literal(),
            origin: at("/query/filter"),
        }],
        time_grains: Vec::new(),
        date_range: Some(QueryDateRange {
            dimension: query_reference(&["orders", "ordered_at"]),
            bounds: DateBounds::Relative(RelativeRange {
                unit: Grain::Month,
                count: 3,
                include_current: None,
            }),
            origin: at("/query/nonportable_extensions/0/data/date_range"),
        }),
        paths: vec![
            QueryPath::new(
                query_reference(&["orders"]),
                query_reference(&["customers"]),
                vec![query_reference(&["orders_to_customers"])],
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
        madrid_time(),
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
    let relation = QueryRelation {
        model: identifier("shop"),
        dataset: identifier("orders"),
        alias: Some(identifier("o")),
        origin: in_sql(1, 37),
    };
    let tree = QueryTree::in_select_order(
        relation.clone(),
        vec![OutputColumn::Metric(first), OutputColumn::Dimension(second)],
        madrid_time(),
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
    assert_eq!(tree.relation(), Some(&relation));
}

#[test]
fn e1_5_the_wide_table_form_has_no_relation() {
    let tree = QueryTree::wide_table(
        Vec::new(),
        Vec::new(),
        madrid_time(),
        QueryTreeParts::default(),
    );
    assert_eq!(tree.relation(), None);
}

fn identifier(text: &str) -> Identifier {
    Identifier::regular(text).unwrap_or_else(|| unreachable!("a regular identifier"))
}

#[test]
fn e1_5_keeps_every_element_with_its_origin() {
    let grained = dimension("ordered_at", "/query/dimensions/0");
    let mut parts = every_element();
    parts.time_grains = vec![TimeGrainChoice {
        dimension: query_reference(&["orders", "ordered_at"]),
        grain: Grain::Month,
        origin: at("/query/nonportable_extensions/0/data/time_grains/0"),
    }];
    let inline = QueryMetric {
        output_name: OutputName::new("double_revenue"),
        kind: QueryMetricKind::Inline(true_literal()),
        origin: at("/query/metrics/1"),
    };
    let tree = QueryTree::wide_table(
        vec![grained],
        vec![metric("revenue", "/query/metrics/0"), inline],
        madrid_time(),
        parts,
    );

    assert_eq!(tree.columns().metrics().count(), 2);
    let grain = tree.time_grains().first().expect("one time grain");
    assert_eq!(grain.dimension, query_reference(&["orders", "ordered_at"]));
    assert_eq!(grain.grain, Grain::Month);
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
        query_reference(&["orders"]),
        query_reference(&["customers"]),
        Vec::new(),
        at("/query/nonportable_extensions/0/data/paths/0"),
    );
    assert!(path.is_none());
}

#[test]
fn e1_5_keeps_the_time_zone_and_the_reference_time() {
    let tree = QueryTree::wide_table(
        vec![dimension("region", "/query/dimensions/0")],
        Vec::new(),
        madrid_time(),
        QueryTreeParts::default(),
    );

    assert_eq!(tree.time().time_zone.as_str(), "Europe/Madrid");
    let reference_time = tree
        .time()
        .reference_time
        .as_ref()
        .expect("a reference time");
    assert_eq!(reference_time.offset.minutes(), 120);
    assert_eq!(reference_time.local.time.hour(), 10);
}

#[test]
fn e1_5_a_reference_keeps_every_part_as_written() {
    let parts = vec![
        ReferencePart::Regular("sales".to_owned()),
        ReferencePart::Delimited("Order Lines".to_owned()),
        ReferencePart::Regular(String::new()),
        ReferencePart::Malformed("\"a\"b\"".to_owned()),
    ];
    let reference = QueryReference::new(parts.clone()).expect("at least one part");

    assert_eq!(reference.parts(), parts.as_slice());
    assert!(QueryReference::new(Vec::new()).is_none());
}

#[test]
fn e1_5_1_output_name_is_the_alias_or_the_last_part_of_the_reference() {
    assert_eq!(OutputName::new("Total Revenue").as_str(), "Total Revenue");
    assert_eq!(
        OutputName::from_reference(&query_reference(&["orders", "region"])).as_str(),
        "region"
    );
    let delimited = QueryReference::new(vec![
        ReferencePart::Regular("orders".to_owned()),
        ReferencePart::Delimited("Net Amount".to_owned()),
    ])
    .expect("two parts");
    assert_eq!(
        OutputName::from_reference(&delimited).as_str(),
        "Net Amount"
    );
    let written_case = query_reference(&["Orders", "Region"]);
    assert_eq!(OutputName::from_reference(&written_case).as_str(), "Region");
}
