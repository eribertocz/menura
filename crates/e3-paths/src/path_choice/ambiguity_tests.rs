//! Unit tests of path choice (E3-3.2, E3-3.3, E2-5 row 17), named after the rule they
//! check.

use menura_representations::bound_query::AuthorizedSubgraph;
use menura_representations::paths::QueryElement;

use super::test_support::{
    bound_path, chosen, date_range_on, error_at, graph_model, only_branch, parts_with_paths,
    revenue_by, shop, step,
};
use crate::PathCode;
use crate::resolve_paths;
use crate::test_support::{dimension, filter, metric, metric_reference, name, query};

#[test]
fn e3_3_2_two_paths_without_a_choice_are_ambiguous_at_the_dimension() {
    let (model, query) = revenue_by(
        vec![
            dimension("ORDERS", "STATUS", 0),
            dimension("CALENDAR", "MONTH", 1),
        ],
        Vec::new(),
    );

    let result = resolve_paths(&query, &model);

    assert_eq!(
        error_at(result),
        (PathCode::AmbiguousPath, "/query/dimensions/1".to_owned())
    );
}

#[test]
fn e3_3_2_a_shorter_path_does_not_remove_the_ambiguity() {
    let model = graph_model(
        vec![metric("REVENUE", &["ORDERS"])],
        &[
            ("ORDERS_STORES", "ORDERS", "STORES"),
            ("ORDERS_STAFF", "ORDERS", "STAFF"),
            ("STAFF_STORES", "STAFF", "STORES"),
        ],
    );
    let parts = parts_with_paths(&model, Vec::new());
    let query = query(
        vec![dimension("STORES", "CITY", 0)],
        vec![metric_reference("REVENUE", 0)],
        parts,
    );

    let result = resolve_paths(&query, &model);

    assert_eq!(error_at(result).0, PathCode::AmbiguousPath);
}

#[test]
fn e3_3_2_a_choice_resolves_the_ambiguity() {
    let (model, query) = revenue_by(
        vec![dimension("CALENDAR", "MONTH", 0)],
        vec![chosen("ORDERS", "CALENDAR", &["ORDER_DATE"], 0)],
    );

    let branches = resolve_paths(&query, &model).expect("one path chosen");

    let branch = only_branch(&branches);
    assert_eq!(
        bound_path(branch, &QueryElement::Dimension(0)),
        [step("ORDER_DATE", "ORDERS", "CALENDAR")]
    );
}

#[test]
fn e3_3_2_an_entry_governs_only_its_pair() {
    let (model, query) = revenue_by(
        vec![
            dimension("CALENDAR", "MONTH", 0),
            dimension("FISCAL_YEARS", "YEAR", 1),
        ],
        vec![chosen("ORDERS", "CALENDAR", &["ORDER_DATE"], 0)],
    );

    let result = resolve_paths(&query, &model);

    assert_eq!(
        error_at(result),
        (PathCode::AmbiguousPath, "/query/dimensions/1".to_owned())
    );
}

#[test]
fn e3_3_2_an_ambiguous_filter_dataset_is_located_at_its_filter() {
    let model = shop();
    let mut parts = parts_with_paths(&model, Vec::new());
    parts.filters = vec![filter(&["CALENDAR"], 0)];
    let query = query(
        vec![dimension("ORDERS", "STATUS", 0)],
        vec![metric_reference("REVENUE", 0)],
        parts,
    );

    let result = resolve_paths(&query, &model);

    assert_eq!(
        error_at(result),
        (PathCode::AmbiguousPath, "/query/filters/0".to_owned())
    );
}

#[test]
fn e3_3_2_an_ambiguous_date_range_dataset_is_located_at_the_date_range() {
    let model = shop();
    let mut parts = parts_with_paths(&model, Vec::new());
    parts.date_range = Some(date_range_on("CALENDAR"));
    let query = query(
        vec![dimension("ORDERS", "STATUS", 0)],
        vec![metric_reference("REVENUE", 0)],
        parts,
    );

    let result = resolve_paths(&query, &model);

    assert_eq!(
        error_at(result),
        (PathCode::AmbiguousPath, "/query/date_range".to_owned())
    );
}

#[test]
fn e3_3_2_paths_outside_the_authorized_subgraph_do_not_count() {
    let model = shop();
    let mut parts = parts_with_paths(&model, Vec::new());
    parts.subgraph = AuthorizedSubgraph::new(
        vec![name("ORDERS"), name("CALENDAR")],
        vec![name("ORDER_DATE")],
    );
    let query = query(
        vec![dimension("CALENDAR", "MONTH", 0)],
        vec![metric_reference("REVENUE", 0)],
        parts,
    );

    let branches = resolve_paths(&query, &model).expect("one path");

    assert_eq!(
        bound_path(only_branch(&branches), &QueryElement::Dimension(0)),
        [step("ORDER_DATE", "ORDERS", "CALENDAR")]
    );
}

#[test]
fn e2_5_row_17_reports_the_first_ambiguous_element_in_query_order() {
    let model = shop();
    let mut parts = parts_with_paths(&model, Vec::new());
    parts.date_range = Some(date_range_on("CALENDAR"));
    let query = query(
        vec![dimension("FISCAL_YEARS", "YEAR", 0)],
        vec![metric_reference("REVENUE", 0)],
        parts,
    );

    let result = resolve_paths(&query, &model);

    assert_eq!(
        error_at(result),
        (PathCode::AmbiguousPath, "/query/dimensions/0".to_owned())
    );
}

#[test]
fn e2_5_row_17_reports_an_ambiguous_path_before_an_unused_entry() {
    let (model, query) = revenue_by(
        vec![dimension("CALENDAR", "MONTH", 0)],
        vec![chosen("ORDERS", "CUSTOMERS", &["ORDERS_CUSTOMERS"], 0)],
    );

    let result = resolve_paths(&query, &model);

    assert_eq!(error_at(result).0, PathCode::AmbiguousPath);
}

#[test]
fn e3_3_3_an_entry_whose_to_is_only_intermediate_is_e3_01() {
    let (model, query) = revenue_by(
        vec![dimension("REGIONS", "NAME", 0)],
        vec![chosen("ORDERS", "CUSTOMERS", &["ORDERS_CUSTOMERS"], 0)],
    );

    let result = resolve_paths(&query, &model);

    assert_eq!(
        error_at(result),
        (PathCode::UnusedPathEntry, "/query/paths/0".to_owned())
    );
}

#[test]
fn e3_3_3_an_entry_whose_from_is_no_home_dataset_is_e3_01() {
    let (model, query) = revenue_by(
        vec![
            dimension("CALENDAR", "MONTH", 0),
            dimension("FISCAL_YEARS", "YEAR", 1),
        ],
        vec![
            chosen("ORDERS", "CALENDAR", &["ORDER_DATE"], 0),
            chosen(
                "ORDERS",
                "FISCAL_YEARS",
                &["ORDER_DATE", "CALENDAR_YEAR"],
                1,
            ),
            chosen("CALENDAR", "FISCAL_YEARS", &["CALENDAR_YEAR"], 2),
        ],
    );

    let result = resolve_paths(&query, &model);

    assert_eq!(
        error_at(result),
        (PathCode::UnusedPathEntry, "/query/paths/2".to_owned())
    );
}

#[test]
fn e3_3_3_a_choice_that_no_ambiguity_requires_is_accepted() {
    let (model, query) = revenue_by(
        vec![dimension("REGIONS", "NAME", 0)],
        vec![chosen(
            "ORDERS",
            "REGIONS",
            &["ORDERS_CUSTOMERS", "CUSTOMERS_REGIONS"],
            0,
        )],
    );

    let branches = resolve_paths(&query, &model).expect("an accepted choice");

    assert_eq!(only_branch(&branches).join_tree.instances().len(), 2);
}
