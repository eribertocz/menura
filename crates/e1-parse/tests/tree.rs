//! E1-3.1 references, E1-5 query tree, E1-5.1 output names and QRY-4 extensions.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_contracts::types::query::{DateBounds, Direction, NullsOrder};
use menura_e1_parse::ParsedQuery;
use menura_representations::columns::OutputColumn;
use menura_representations::query_tree::{
    QueryDimension, QueryDimensionKind, QueryMetric, QueryMetricKind, QueryReference, ReferencePart,
};
use serde_json::{Value, json};
use support::{accepted, extension_data, wide_table_query, with_extension};

#[cfg(test)]
fn with_columns(dimensions: &Value, metrics: &Value) -> ParsedQuery {
    let mut document = wide_table_query();
    document["query"] = json!({ "dimensions": *dimensions, "metrics": *metrics });
    accepted(&document)
}

#[cfg(test)]
fn dimensions(parsed: &ParsedQuery) -> Vec<&QueryDimension> {
    parsed.tree.columns().dimensions().collect()
}

#[cfg(test)]
fn metrics(parsed: &ParsedQuery) -> Vec<&QueryMetric> {
    parsed.tree.columns().metrics().collect()
}

#[cfg(test)]
fn field_reference(dimension: &QueryDimension) -> &QueryReference {
    match &dimension.kind {
        QueryDimensionKind::Field { reference, .. } => reference,
        QueryDimensionKind::Expression(_) => {
            unreachable!("the wide-table form has field dimensions")
        }
    }
}

#[cfg(test)]
fn regular(text: &str) -> ReferencePart {
    ReferencePart::Regular(text.to_owned())
}

#[cfg(test)]
fn delimited(text: &str) -> ReferencePart {
    ReferencePart::Delimited(text.to_owned())
}

#[cfg(test)]
fn malformed(text: &str) -> ReferencePart {
    ReferencePart::Malformed(text.to_owned())
}

#[cfg(test)]
fn parts_of(reference: &str) -> Vec<ReferencePart> {
    let parsed = with_columns(&json!([reference]), &json!([]));
    let dimension = dimensions(&parsed).first().copied().cloned();
    let dimension = dimension.expect("one dimension");
    field_reference(&dimension).parts().to_vec()
}

#[test]
fn e1_3_1_splits_a_reference_into_regular_parts() {
    assert_eq!(
        parts_of("orders.region"),
        [regular("orders"), regular("region")]
    );
}

#[test]
fn e1_3_1_keeps_three_parts_without_error() {
    assert_eq!(
        parts_of("a.b.c"),
        [regular("a"), regular("b"), regular("c")]
    );
}

#[test]
fn e1_3_1_reads_a_delimited_part_without_its_quotes() {
    assert_eq!(
        parts_of("orders.\"Net Amount\""),
        [regular("orders"), delimited("Net Amount")]
    );
}

#[test]
fn e1_3_1_does_not_split_at_a_dot_inside_a_delimited_part() {
    assert_eq!(
        parts_of("\"my.orders\".\"say \"\"hi\"\"\""),
        [delimited("my.orders"), delimited("say \"hi\"")]
    );
}

#[test]
fn e1_3_1_keeps_empty_and_malformed_parts() {
    assert_eq!(parts_of("orders."), [regular("orders"), regular("")]);
    assert_eq!(
        parts_of("orders.\"a"),
        [regular("orders"), malformed("\"a")]
    );
    assert_eq!(parts_of("\"\".x"), [malformed("\"\""), regular("x")]);
    assert_eq!(parts_of("\"a\"b\""), [malformed("\"a\"b\"")]);
}

#[test]
fn e1_3_1_a_bare_metric_is_one_part() {
    let parsed = with_columns(&json!([]), &json!(["revenue"]));
    let metric = metrics(&parsed).first().copied().cloned();
    let Some(QueryMetric {
        kind: QueryMetricKind::Reference(reference),
        ..
    }) = metric
    else {
        panic!("expected a metric reference");
    };
    assert_eq!(reference.parts(), [regular("revenue")]);
}

#[test]
fn e1_5_1_output_name_is_the_last_part_as_written() {
    let parsed = with_columns(
        &json!(["orders.\"Net Amount\"", "Orders.Region"]),
        &json!(["revenue"]),
    );
    let names: Vec<&str> = dimensions(&parsed)
        .iter()
        .map(|dimension| dimension.output_name.as_str())
        .collect();
    assert_eq!(names, ["Net Amount", "Region"]);
}

#[test]
fn e1_5_1_output_name_is_the_alias_when_written() {
    let parsed = with_columns(
        &json!([{ "field": "orders.region", "alias": "Sales Region" }]),
        &json!([{ "metric": "revenue", "alias": "total" }]),
    );
    assert_eq!(
        dimensions(&parsed).first().map(|d| d.output_name.as_str()),
        Some("Sales Region")
    );
    assert_eq!(
        metrics(&parsed).first().map(|m| m.output_name.as_str()),
        Some("total")
    );
}

#[test]
fn e1_5_puts_dimensions_before_metrics_with_their_origins() {
    let mut document = wide_table_query();
    document["query"] = json!({
        "metrics": ["revenue", "orders.count"],
        "dimensions": ["orders.region", "orders.status"]
    });
    let parsed = accepted(&document);
    let columns: Vec<(&str, &str)> = parsed
        .tree
        .columns()
        .iter()
        .map(|column| match column {
            OutputColumn::Dimension(dimension) => {
                (dimension.output_name.as_str(), dimension.origin.pointer())
            }
            OutputColumn::Metric(metric) => (metric.output_name.as_str(), metric.origin.pointer()),
        })
        .collect();
    assert_eq!(
        columns,
        [
            ("region", "/query/dimensions/0"),
            ("status", "/query/dimensions/1"),
            ("revenue", "/query/metrics/0"),
            ("count", "/query/metrics/1"),
        ]
    );
}

#[test]
fn qry_4_parses_date_range_paths_filters_order_and_limit_with_their_origins() {
    let document = with_extension(&extension_data(&json!({
        "date_range": { "dimension": "orders.ordered_at", "from": "2026-01-01", "to": "2026-03-31" },
        "paths": [{ "from": "orders", "to": "customers", "relationships": ["orders_to_customers"] }],
        "filters": ["orders.amount > 1"],
        "order_by": [{ "output": "region", "direction": "desc", "nulls": "last" }],
        "limit": 10
    })));
    let tree = accepted(&document).tree;
    let data = "/query/nonportable_extensions/0/data";

    let range = tree.date_range().unwrap_or_else(|| panic!("a date range"));
    assert_eq!(range.origin.pointer(), format!("{data}/date_range"));
    assert_eq!(
        range.dimension.parts(),
        [regular("orders"), regular("ordered_at")]
    );
    let absolute = DateBounds::Absolute {
        from: "2026-01-01".to_owned(),
        to: "2026-03-31".to_owned(),
    };
    assert_eq!(range.bounds, absolute);

    let path = tree.paths().first().unwrap_or_else(|| panic!("a path"));
    assert_eq!(path.origin().pointer(), format!("{data}/paths/0"));
    assert_eq!(path.to().parts(), [regular("customers")]);
    assert_eq!(path.relationships().len(), 1);

    let filter = tree.filters().first().unwrap_or_else(|| panic!("a filter"));
    assert_eq!(filter.origin.pointer(), format!("{data}/filters/0"));

    let order = tree
        .order()
        .first()
        .unwrap_or_else(|| panic!("an order item"));
    assert_eq!(order.output.as_str(), "region");
    assert_eq!(
        (order.direction, order.nulls),
        (Direction::Desc, NullsOrder::Last)
    );
    assert_eq!(order.origin.pointer(), format!("{data}/order_by/0"));

    let limit = tree.limit().unwrap_or_else(|| panic!("a limit"));
    assert_eq!(
        (limit.rows, limit.origin.pointer()),
        (10, format!("{data}/limit").as_str())
    );
}

#[test]
fn qry_4_keeps_a_relative_date_range_as_written() {
    let document = with_extension(&extension_data(&json!({
        "date_range": { "dimension": "orders.ordered_at", "relative": { "unit": "month", "count": 3 } }
    })));
    let tree = accepted(&document).tree;
    let Some(DateBounds::Relative(relative)) = tree.date_range().map(|range| range.bounds.clone())
    else {
        panic!("a relative range");
    };
    assert_eq!(
        (relative.unit, relative.count, relative.include_current),
        (Grain::Month, 3, None)
    );
}
