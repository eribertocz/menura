//! Unit tests of reachability, anchors and the E3 diagnostics, named after the rule they
//! check.

use menura_model::expression::DateValue;
use menura_model::internal_model::{FieldPath, InternalModel};
use menura_representations::bound_query::{
    BoundDateRange, BoundDimension, BoundFilter, BoundMetric, BoundQuery, BoundQueryParts,
};
use menura_representations::origin::QueryLocation;

use crate::test_support::{
    at, connected_model, dimension, filter, metric, metric_reference, name, query, subgraph,
};
use crate::{PathCode, PathError, resolve_paths};

/// Every relationship of the fixture: returns to orders to customers to regions, accounts and
/// profiles both ways; warehouses has none.
const RELATIONSHIPS: [&str; 5] = [
    "RETURNS_ORDERS",
    "ORDERS_CUSTOMERS",
    "CUSTOMERS_REGIONS",
    "ACCOUNTS_PROFILES",
    "PROFILES_ACCOUNTS",
];

const DATE_RANGE: &str = "/query/nonportable_extensions/0/data/date_range";

fn fixture() -> InternalModel {
    connected_model(
        Vec::new(),
        vec![
            metric("AMOUNT", &["ORDERS"]),
            metric("NET", &["ORDERS", "RETURNS"]),
            metric("REFUND", &["RETURNS"]),
        ],
        &[
            ("RETURNS_ORDERS", "RETURNS", "ORDERS"),
            ("ORDERS_CUSTOMERS", "ORDERS", "CUSTOMERS"),
            ("CUSTOMERS_REGIONS", "CUSTOMERS", "REGIONS"),
            ("ACCOUNTS_PROFILES", "ACCOUNTS", "PROFILES"),
            ("PROFILES_ACCOUNTS", "PROFILES", "ACCOUNTS"),
        ],
    )
}

fn authorized_query(
    dimensions: Vec<BoundDimension>,
    metrics: Vec<BoundMetric>,
    filters: Vec<BoundFilter>,
    date_range: Option<BoundDateRange>,
    relationships: &[&str],
) -> BoundQuery {
    let parts = BoundQueryParts {
        filters,
        date_range,
        subgraph: subgraph(relationships),
        ..BoundQueryParts::default()
    };
    query(dimensions, metrics, parts)
}

fn date_range_on(dataset: &str) -> BoundDateRange {
    let date = DateValue::new(2026, 1, 1).expect("a date");
    BoundDateRange {
        field: FieldPath {
            dataset: name(dataset),
            field: name("DAY"),
        },
        from: date,
        to: date,
        origin: at(DATE_RANGE),
    }
}

fn metrics(names: &[&str]) -> Vec<BoundMetric> {
    names
        .iter()
        .enumerate()
        .map(|(index, metric_name)| metric_reference(metric_name, index))
        .collect()
}

/// The code and location of the E3 error of `query`, or `None` when E3 reports none.
fn first_error(query: &BoundQuery) -> Option<(PathCode, QueryLocation)> {
    match resolve_paths(query, &fixture()) {
        Err(PathError::Diagnostic(diagnostic)) => Some((diagnostic.code, diagnostic.location)),
        _ => None,
    }
}

fn error_at(code: PathCode, pointer: &str) -> (PathCode, QueryLocation) {
    (code, at(pointer))
}

#[test]
fn e3_3_1_a_step_never_goes_against_the_many_to_one_direction() {
    let query = authorized_query(
        vec![dimension("RETURNS", "REASON", 0)],
        metrics(&["AMOUNT"]),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::NoPath, "/query/dimensions/0"))
    );
}

#[test]
fn e3_3_1_reachability_uses_only_authorized_relationships() {
    let authorized = ["RETURNS_ORDERS", "ORDERS_CUSTOMERS"];
    let query = authorized_query(
        vec![dimension("REGIONS", "NAME", 0)],
        metrics(&["AMOUNT"]),
        Vec::new(),
        None,
        &authorized,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::NoPath, "/query/dimensions/0"))
    );
}

#[test]
fn e3_3_1_a_path_of_several_steps_reaches_its_dataset() {
    let query = authorized_query(
        vec![dimension("REGIONS", "NAME", 0)],
        metrics(&["REFUND"]),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(first_error(&query), None);
}

#[test]
fn e3_3_4_a_metric_reaches_a_dataset_only_from_every_component_home() {
    let query = authorized_query(
        vec![dimension("RETURNS", "REASON", 0)],
        metrics(&["NET"]),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::NoPath, "/query/dimensions/0"))
    );
}

#[test]
fn e3_3_5_a_dimension_no_metric_reaches_is_e_no_path() {
    let query = authorized_query(
        vec![dimension("WAREHOUSES", "CITY", 0)],
        metrics(&["AMOUNT", "REFUND"]),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::NoPath, "/query/dimensions/0"))
    );
}

#[test]
fn e3_3_5_a_dimension_only_some_metrics_reach_is_partial_reachability() {
    let query = authorized_query(
        vec![dimension("RETURNS", "REASON", 0)],
        metrics(&["AMOUNT", "REFUND"]),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(
            PathCode::PartialDimensionReachability,
            "/query/dimensions/0"
        ))
    );
}

#[test]
fn e3_3_6_a_filter_dataset_some_metric_cannot_reach_is_located_at_its_filter() {
    let query = authorized_query(
        vec![dimension("ORDERS", "STATUS", 0)],
        metrics(&["AMOUNT", "REFUND"]),
        vec![filter(&["ORDERS"], 0), filter(&["RETURNS"], 1)],
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(
            PathCode::UnsupportedFilterPath,
            "/query/filters/1"
        ))
    );
}

#[test]
fn e3_3_6_a_filter_that_reads_no_dataset_needs_no_check() {
    let query = authorized_query(
        vec![dimension("ORDERS", "STATUS", 0)],
        metrics(&["AMOUNT"]),
        vec![filter(&[], 0)],
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(first_error(&query), None);
}

#[test]
fn e3_3_1a_the_date_range_dataset_is_checked_as_a_filter_dataset_at_its_entry() {
    let query = authorized_query(
        vec![dimension("ORDERS", "STATUS", 0)],
        metrics(&["AMOUNT"]),
        Vec::new(),
        Some(date_range_on("RETURNS")),
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::UnsupportedFilterPath, DATE_RANGE))
    );
}

#[test]
fn e3_3_1a_the_date_range_comes_after_the_filters_in_query_order() {
    let query = authorized_query(
        vec![dimension("ORDERS", "STATUS", 0)],
        metrics(&["AMOUNT"]),
        vec![filter(&["WAREHOUSES"], 0)],
        Some(date_range_on("RETURNS")),
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(
            PathCode::UnsupportedFilterPath,
            "/query/filters/0"
        ))
    );
}

#[test]
fn e2_5_e_no_path_is_checked_over_every_dimension_before_partial_reachability() {
    let query = authorized_query(
        vec![
            dimension("RETURNS", "REASON", 0),
            dimension("WAREHOUSES", "CITY", 1),
        ],
        metrics(&["AMOUNT", "REFUND"]),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::NoPath, "/query/dimensions/1"))
    );
}

#[test]
fn e2_5_partial_reachability_comes_before_an_unsupported_filter_path() {
    let query = authorized_query(
        vec![dimension("RETURNS", "REASON", 0)],
        metrics(&["AMOUNT", "REFUND"]),
        vec![filter(&["WAREHOUSES"], 0)],
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(
            PathCode::PartialDimensionReachability,
            "/query/dimensions/0"
        ))
    );
}

#[test]
fn e3_5_1_a_dimension_dataset_that_reaches_the_others_is_an_anchor() {
    let query = authorized_query(
        vec![
            dimension("CUSTOMERS", "NAME", 0),
            dimension("ORDERS", "STATUS", 1),
        ],
        Vec::new(),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    let branches = resolve_paths(&query, &fixture()).expect("one anchor");

    let homes: Vec<&str> = branches
        .as_slice()
        .iter()
        .map(|branch| branch.home.as_str())
        .collect();
    assert_eq!(homes, ["ORDERS"]);
}

#[test]
fn e3_5_1_without_an_anchor_is_e_no_anchor_at_the_dimensions() {
    let query = authorized_query(
        vec![
            dimension("CUSTOMERS", "NAME", 0),
            dimension("WAREHOUSES", "CITY", 1),
        ],
        Vec::new(),
        vec![filter(&["RETURNS"], 0)],
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::NoAnchor, "/query/dimensions"))
    );
}

#[test]
fn e3_5_1_more_than_one_anchor_is_e_ambiguous_path_at_the_dimensions() {
    let query = authorized_query(
        vec![
            dimension("ACCOUNTS", "PLAN", 0),
            dimension("PROFILES", "SEGMENT", 1),
        ],
        Vec::new(),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::AmbiguousPath, "/query/dimensions"))
    );
}

#[test]
fn e3_5_2_a_filter_dataset_the_anchor_cannot_reach_is_located_at_its_filter() {
    let query = authorized_query(
        vec![dimension("ORDERS", "STATUS", 0)],
        Vec::new(),
        vec![filter(&["RETURNS"], 0)],
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(
            PathCode::UnsupportedFilterPath,
            "/query/filters/0"
        ))
    );
}

#[test]
fn e3_5_2_a_date_range_the_anchor_cannot_reach_is_located_at_its_entry() {
    let query = authorized_query(
        vec![dimension("ORDERS", "STATUS", 0)],
        Vec::new(),
        Vec::new(),
        Some(date_range_on("RETURNS")),
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(PathCode::UnsupportedFilterPath, DATE_RANGE))
    );
}

#[test]
fn e3_5_2_with_several_anchors_the_filter_check_comes_before_the_ambiguity() {
    let query = authorized_query(
        vec![
            dimension("ACCOUNTS", "PLAN", 0),
            dimension("PROFILES", "SEGMENT", 1),
        ],
        Vec::new(),
        vec![filter(&["WAREHOUSES"], 0)],
        None,
        &RELATIONSHIPS,
    );

    assert_eq!(
        first_error(&query),
        Some(error_at(
            PathCode::UnsupportedFilterPath,
            "/query/filters/0"
        ))
    );
}

#[test]
fn e3_6_each_code_is_written_as_the_specification_names_it() {
    let codes = [
        (PathCode::NoPath, "E_NO_PATH"),
        (
            PathCode::PartialDimensionReachability,
            "E_PARTIAL_DIMENSION_REACHABILITY",
        ),
        (PathCode::NoAnchor, "E_NO_ANCHOR"),
        (PathCode::UnsupportedFilterPath, "E_UNSUPPORTED_FILTER_PATH"),
        (PathCode::AmbiguousPath, "E_AMBIGUOUS_PATH"),
    ];
    for (code, text) in codes {
        assert_eq!(code.as_str(), text);
    }
}

#[test]
fn e3_6_a_diagnostic_is_located_in_the_query_document() {
    let query = authorized_query(
        vec![dimension("WAREHOUSES", "CITY", 0)],
        metrics(&["AMOUNT"]),
        Vec::new(),
        None,
        &RELATIONSHIPS,
    );
    let Err(PathError::Diagnostic(diagnostic)) = resolve_paths(&query, &fixture()) else {
        panic!("an E3 error");
    };

    let written = diagnostic.to_diagnostic();

    assert_eq!(written.code, "E_NO_PATH");
    let location = written.location.expect("a location");
    assert_eq!(location.pointer, "/query/dimensions/0");
    assert!(written.message.contains("WAREHOUSES"));
}
