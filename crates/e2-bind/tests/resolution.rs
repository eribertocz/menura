//! E2-1, E2-2 and E2-3: every reference of the query tree resolves against the model.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::{BindCode, ResolvedName};
use menura_model::internal_model::FieldPath;
use menura_representations::bound_query::{BoundDimensionKind, BoundMetricKind, MetricPath};
use menura_representations::columns::OutputColumn;
use menura_representations::query_tree::{QueryTreeParts, ReferencePart};

use support::model::name;
use support::query::{
    column, date_range, dimension, dimension_reference, dimension_with_grain, equals_text, filter,
    metric, parts, path, tree,
};
use support::{first_error, run};

const ANALYST: &[&str] = &["analyst"];

fn field(dataset: &str, field: &str) -> FieldPath {
    FieldPath {
        dataset: name(dataset),
        field: name(field),
    }
}

#[test]
fn e2_1_every_name_resolves_against_the_model() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    assert!(run(&query, ANALYST).is_ok());
}

fn four_columns() -> menura_representations::query_tree::QueryTree {
    tree(
        vec![
            dimension_with_grain(0, "orders.ordered_at", Grain::Month),
            dimension(1, "customers.region"),
        ],
        vec![metric(0, "revenue"), metric(1, "orders.order_total")],
        QueryTreeParts::default(),
    )
}

#[test]
fn e2_2_every_column_points_to_its_model_element_in_order() {
    let authorized = run(&four_columns(), ANALYST).expect("authorized");
    let columns: Vec<_> = authorized.columns().iter().collect();
    let [
        OutputColumn::Dimension(ordered_at),
        OutputColumn::Dimension(region),
        OutputColumn::Metric(revenue),
        OutputColumn::Metric(order_total),
    ] = columns.as_slice()
    else {
        panic!("two dimensions, then two metrics: {columns:?}");
    };
    let grained = BoundDimensionKind::Field {
        field: field("ORDERS", "ORDERED_AT"),
        time_grain: Some(Grain::Month),
    };
    assert_eq!(ordered_at.kind, grained);
    let plain = BoundDimensionKind::Field {
        field: field("CUSTOMERS", "REGION"),
        time_grain: None,
    };
    assert_eq!(region.kind, plain);
    let semantic = MetricPath::SemanticModel(name("REVENUE"));
    assert_eq!(revenue.kind, BoundMetricKind::Reference(semantic));
    let scoped = MetricPath::Dataset {
        dataset: name("ORDERS"),
        metric: name("ORDER_TOTAL"),
    };
    assert_eq!(order_total.kind, BoundMetricKind::Reference(scoped));
}

#[test]
fn e2_2_every_column_keeps_its_output_name_and_origin() {
    let authorized = run(&four_columns(), ANALYST).expect("authorized");
    let named: Vec<(&str, &str)> = authorized
        .columns()
        .iter()
        .map(|column| match column {
            OutputColumn::Dimension(dimension) => {
                (dimension.output_name.as_str(), dimension.origin.pointer())
            }
            OutputColumn::Metric(metric) => (metric.output_name.as_str(), metric.origin.pointer()),
        })
        .collect();
    let expected = [
        ("ordered_at", "/query/dimensions/0"),
        ("region", "/query/dimensions/1"),
        ("revenue", "/query/metrics/0"),
        ("order_total", "/query/metrics/1"),
    ];
    assert_eq!(named, expected);
}

#[test]
fn e2_2_filters_date_range_and_paths_point_to_model_elements() {
    let parts = QueryTreeParts {
        filters: vec![filter(
            "/query/filter",
            equals_text(column("customers.region")),
        )],
        date_range: Some(date_range(
            "/query/nonportable_extensions/0/data/date_range",
            "orders.ordered_at",
        )),
        paths: vec![path(
            "/query/nonportable_extensions/1/data/paths/0",
            "orders",
            "customers",
            &["orders_to_customers"],
        )],
        ..QueryTreeParts::default()
    };
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        parts,
    );
    let authorized = run(&query, ANALYST).expect("authorized");
    let filter = &authorized.filters()[0];
    assert_eq!(filter.datasets, vec![name("CUSTOMERS")]);
    assert_eq!(filter.origin.pointer(), "/query/filter");
    let range = authorized.date_range().expect("a date range");
    assert_eq!(
        range.target,
        ResolvedName::Field(field("ORDERS", "ORDERED_AT"))
    );
    let bound_path = &authorized.paths()[0];
    assert_eq!(bound_path.from(), &name("ORDERS"));
    assert_eq!(bound_path.to(), &name("CUSTOMERS"));
    assert_eq!(bound_path.relationships(), &[name("ORDERS_TO_CUSTOMERS")]);
}

#[test]
fn e2_3_1_regular_parts_ignore_case_and_delimited_parts_compare_exactly() {
    let regular = tree(
        vec![dimension(0, "Orders.REGION")],
        vec![],
        QueryTreeParts::default(),
    );
    assert!(run(&regular, ANALYST).is_ok());
    let delimited = |text: &str| {
        let reference = parts(vec![
            ReferencePart::Regular("customers".to_owned()),
            ReferencePart::Delimited(text.to_owned()),
        ]);
        tree(
            vec![dimension_reference(0, reference)],
            vec![],
            QueryTreeParts::default(),
        )
    };
    assert!(run(&delimited("Segment"), ANALYST).is_ok());
    assert_eq!(
        first_error(&delimited("SEGMENT"), ANALYST).0,
        BindCode::NameNotFound
    );
    let upper_delimited = parts(vec![
        ReferencePart::Delimited("ORDERS".to_owned()),
        ReferencePart::Regular("region".to_owned()),
    ]);
    let query = tree(
        vec![dimension_reference(0, upper_delimited)],
        vec![],
        QueryTreeParts::default(),
    );
    assert!(run(&query, ANALYST).is_ok());
}

#[test]
fn e2_3_2_an_unknown_reference_is_name_not_found_at_the_reference() {
    let query = tree(
        vec![dimension(0, "orders.region"), dimension(1, "orders.colour")],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        (BindCode::NameNotFound, "/query/dimensions/1".to_owned())
    );
}

#[test]
fn e2_3_2_a_three_part_reference_is_name_not_found() {
    let query = tree(
        vec![dimension(0, "sales.orders.region")],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        (BindCode::NameNotFound, "/query/dimensions/0".to_owned())
    );
}

#[test]
fn e2_3_2_a_reference_with_an_empty_part_is_name_not_found() {
    for text in ["orders.", ".region", "orders..region"] {
        let query = tree(vec![], vec![metric(0, text)], QueryTreeParts::default());
        assert_eq!(
            first_error(&query, ANALYST),
            (BindCode::NameNotFound, "/query/metrics/0".to_owned()),
            "{text}"
        );
    }
}

#[test]
fn e2_3_2_a_malformed_delimited_part_is_name_not_found() {
    let reference = parts(vec![
        ReferencePart::Regular("orders".to_owned()),
        ReferencePart::Malformed("\"region".to_owned()),
    ]);
    let query = tree(
        vec![dimension_reference(0, reference)],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(first_error(&query, ANALYST).0, BindCode::NameNotFound);
}

#[test]
fn e2_3_2_a_one_part_dimension_names_a_metric_of_the_semantic_model_only() {
    let query = tree(
        vec![dimension(0, "region")],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(first_error(&query, ANALYST).0, BindCode::NameNotFound);
}

#[test]
fn e2_3_2_filter_references_resolve() {
    let parts = QueryTreeParts {
        filters: vec![filter(
            "/query/filter",
            equals_text(column("orders.colour")),
        )],
        ..QueryTreeParts::default()
    };
    let query = tree(vec![dimension(0, "orders.region")], vec![], parts);
    assert_eq!(
        first_error(&query, ANALYST),
        (BindCode::NameNotFound, "/query/filter".to_owned())
    );
}

#[test]
fn e2_3_2_the_date_range_reference_resolves() {
    let parts = QueryTreeParts {
        date_range: Some(date_range(
            "/query/nonportable_extensions/0/data/date_range",
            "orders.shipped_at",
        )),
        ..QueryTreeParts::default()
    };
    let query = tree(vec![dimension(0, "orders.region")], vec![], parts);
    assert_eq!(
        first_error(&query, ANALYST),
        (
            BindCode::NameNotFound,
            "/query/nonportable_extensions/0/data/date_range".to_owned()
        )
    );
}

#[test]
fn e2_3_3_a_dimension_naming_a_metric_is_metric_in_dimensions() {
    let query = tree(
        vec![dimension(0, "revenue")],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        (
            BindCode::MetricInDimensions,
            "/query/dimensions/0".to_owned()
        )
    );
    let dataset_metric = tree(
        vec![dimension(0, "orders.order_total")],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&dataset_metric, ANALYST).0,
        BindCode::MetricInDimensions
    );
}

#[test]
fn e2_3_3_a_metric_naming_a_field_is_field_in_metrics() {
    let query = tree(
        vec![],
        vec![metric(0, "orders.amount")],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        (BindCode::FieldInMetrics, "/query/metrics/0".to_owned())
    );
}

#[test]
fn e2_3_5_every_relationship_of_a_path_exists() {
    let parts = QueryTreeParts {
        paths: vec![path(
            "/query/nonportable_extensions/0/data/paths/0",
            "orders",
            "customers",
            &["orders_to_customers", "orders_to_nowhere"],
        )],
        ..QueryTreeParts::default()
    };
    let query = tree(
        vec![dimension(0, "customers.region")],
        vec![metric(0, "revenue")],
        parts,
    );
    assert_eq!(
        first_error(&query, ANALYST),
        (
            BindCode::NameNotFound,
            "/query/nonportable_extensions/0/data/paths/0".to_owned()
        )
    );
}

#[test]
fn e2_3_5_the_datasets_of_a_path_exist() {
    let parts = QueryTreeParts {
        paths: vec![path(
            "/query/nonportable_extensions/0/data/paths/0",
            "orders",
            "clients",
            &["orders_to_customers"],
        )],
        ..QueryTreeParts::default()
    };
    let query = tree(vec![dimension(0, "customers.region")], vec![], parts);
    assert_eq!(first_error(&query, ANALYST).0, BindCode::NameNotFound);
}
