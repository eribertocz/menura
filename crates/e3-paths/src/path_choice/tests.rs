//! Unit tests of path choice, join trees and branches (E3-3.2, E3-3.3, E3-4, QRY-4.3),
//! named after the rule they check.

use menura_model::expression::DateValue;
use menura_model::internal_model::FieldPath;
use menura_representations::bound_query::{AuthorizedSubgraph, BoundDateRange, BoundPath};
use menura_representations::paths::{Branch, Branches, ComponentKey, JoinStep, QueryElement};

use super::test_support::{chosen, edge, graph_model, parts_with_paths, step};
use super::{BranchHome, PathChoiceCode, PathChoiceError, choose_paths, component_homes};
use crate::test_support::{at, dimension, filter, metric, metric_reference, name, query};

/// `ORDERS` reaches `CALENDAR` through `SHIP_DATE` and through `ORDER_DATE`, and `CALENDAR`
/// reaches `FISCAL_YEARS`; `ORDERS` also reaches `CUSTOMERS`, which reaches `REGIONS`.
fn shop() -> menura_model::internal_model::InternalModel {
    graph_model(
        Vec::new(),
        vec![
            metric("REVENUE", &["ORDERS"]),
            metric("REFUNDS", &["RETURNS"]),
            metric("MIXED", &["RETURNS", "ORDERS"]),
        ],
        vec![
            edge("SHIP_DATE", "ORDERS", "CALENDAR"),
            edge("ORDER_DATE", "ORDERS", "CALENDAR"),
            edge("CALENDAR_YEAR", "CALENDAR", "FISCAL_YEARS"),
            edge("ORDERS_CUSTOMERS", "ORDERS", "CUSTOMERS"),
            edge("CUSTOMERS_REGIONS", "CUSTOMERS", "REGIONS"),
            edge("RETURNS_ORDERS", "RETURNS", "ORDERS"),
        ],
    )
}

fn home(dataset: &str, components: &[(usize, usize)]) -> BranchHome {
    BranchHome {
        home: name(dataset),
        components: components
            .iter()
            .map(|&(metric, component)| ComponentKey { metric, component })
            .collect(),
    }
}

fn error_at(result: Result<Branches, PathChoiceError>) -> (PathChoiceCode, String) {
    let error = result.expect_err("a path choice error");
    let code = error.code().expect("a specification code");
    let origin = error.origin().expect("a location");
    (code, origin.pointer().to_owned())
}

fn only_branch(branches: &Branches) -> &Branch {
    match branches.as_slice() {
        [branch] => branch,
        other => panic!("expected one branch, got {}", other.len()),
    }
}

fn bound_path(branch: &Branch, element: &QueryElement) -> Vec<JoinStep> {
    let tree = &branch.join_tree;
    tree.bound_instance(element)
        .map(|id| tree.path_to(id).into_iter().cloned().collect())
        .unwrap_or_default()
}

fn date_range_on(dataset: &str) -> BoundDateRange {
    BoundDateRange {
        field: FieldPath {
            dataset: name(dataset),
            field: name("DAY"),
        },
        from: DateValue::new(2024, 1, 1).expect("a valid date"),
        to: DateValue::new(2024, 12, 31).expect("a valid date"),
        origin: at("/query/date_range"),
    }
}

fn revenue_by(
    dimensions: Vec<menura_representations::bound_query::BoundDimension>,
    paths: Vec<BoundPath>,
) -> (
    menura_model::internal_model::InternalModel,
    menura_representations::bound_query::BoundQuery,
) {
    let model = shop();
    let parts = parts_with_paths(&model, paths);
    let query = query(dimensions, vec![metric_reference("REVENUE", 0)], parts);
    (model, query)
}

#[test]
fn e3_3_2_two_paths_without_a_choice_are_ambiguous_at_the_dimension() {
    let (model, query) = revenue_by(
        vec![
            dimension("ORDERS", "STATUS", 0),
            dimension("CALENDAR", "MONTH", 1),
        ],
        Vec::new(),
    );

    let result = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]);

    assert_eq!(
        error_at(result),
        (
            PathChoiceCode::AmbiguousPath,
            "/query/dimensions/1".to_owned()
        )
    );
}

#[test]
fn e3_3_2_a_shorter_path_does_not_remove_the_ambiguity() {
    let model = graph_model(
        Vec::new(),
        vec![metric("REVENUE", &["ORDERS"])],
        vec![
            edge("ORDERS_STORES", "ORDERS", "STORES"),
            edge("ORDERS_STAFF", "ORDERS", "STAFF"),
            edge("STAFF_STORES", "STAFF", "STORES"),
        ],
    );
    let parts = parts_with_paths(&model, Vec::new());
    let query = query(
        vec![dimension("STORES", "CITY", 0)],
        vec![metric_reference("REVENUE", 0)],
        parts,
    );

    let result = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]);

    assert_eq!(error_at(result).0, PathChoiceCode::AmbiguousPath);
}

#[test]
fn e3_3_2_a_choice_resolves_the_ambiguity() {
    let (model, query) = revenue_by(
        vec![dimension("CALENDAR", "MONTH", 0)],
        vec![chosen("ORDERS", "CALENDAR", &["ORDER_DATE"], 0)],
    );

    let branches =
        choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]).expect("one path chosen");

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

    let result = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]);

    assert_eq!(
        error_at(result),
        (
            PathChoiceCode::AmbiguousPath,
            "/query/dimensions/1".to_owned()
        )
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

    let result = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]);

    assert_eq!(
        error_at(result),
        (PathChoiceCode::AmbiguousPath, "/query/filters/0".to_owned())
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

    let result = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]);

    assert_eq!(
        error_at(result),
        (
            PathChoiceCode::AmbiguousPath,
            "/query/date_range".to_owned()
        )
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

    let branches = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]).expect("one path");

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

    let result = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]);

    assert_eq!(
        error_at(result),
        (
            PathChoiceCode::AmbiguousPath,
            "/query/dimensions/0".to_owned()
        )
    );
}

#[test]
fn e2_5_row_17_reports_an_ambiguous_path_before_an_unused_entry() {
    let (model, query) = revenue_by(
        vec![dimension("CALENDAR", "MONTH", 0)],
        vec![chosen("ORDERS", "CUSTOMERS", &["ORDERS_CUSTOMERS"], 0)],
    );

    let result = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]);

    assert_eq!(error_at(result).0, PathChoiceCode::AmbiguousPath);
}

#[test]
fn e3_3_3_an_entry_whose_to_is_only_intermediate_is_e3_01() {
    let (model, query) = revenue_by(
        vec![dimension("REGIONS", "NAME", 0)],
        vec![chosen("ORDERS", "CUSTOMERS", &["ORDERS_CUSTOMERS"], 0)],
    );

    let result = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]);

    assert_eq!(
        error_at(result),
        (PathChoiceCode::UnusedPathEntry, "/query/paths/0".to_owned())
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

    let result = choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]);

    assert_eq!(
        error_at(result),
        (PathChoiceCode::UnusedPathEntry, "/query/paths/2".to_owned())
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

    let branches =
        choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]).expect("an accepted choice");

    assert_eq!(only_branch(&branches).join_tree.instances().len(), 2);
}

#[test]
fn qry_4_3_the_chosen_path_is_used_for_every_element_of_its_pair() {
    let model = shop();
    let mut parts = parts_with_paths(
        &model,
        vec![chosen("ORDERS", "CALENDAR", &["SHIP_DATE"], 0)],
    );
    parts.date_range = Some(date_range_on("CALENDAR"));
    let query = query(
        vec![dimension("CALENDAR", "MONTH", 0)],
        vec![metric_reference("REVENUE", 0)],
        parts,
    );

    let branches =
        choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]).expect("one path chosen");

    let branch = only_branch(&branches);
    let shipped = [step("SHIP_DATE", "ORDERS", "CALENDAR")];
    assert_eq!(bound_path(branch, &QueryElement::Dimension(0)), shipped);
    assert_eq!(bound_path(branch, &QueryElement::DateRange), shipped);
    assert_eq!(branch.join_tree.instances().len(), 1);
}

#[test]
fn e3_4_2_a_role_playing_reaches_one_dataset_as_two_instances() {
    let (model, query) = revenue_by(
        vec![
            dimension("CALENDAR", "MONTH", 0),
            dimension("FISCAL_YEARS", "YEAR", 1),
        ],
        vec![
            chosen("ORDERS", "CALENDAR", &["ORDER_DATE"], 0),
            chosen("ORDERS", "FISCAL_YEARS", &["SHIP_DATE", "CALENDAR_YEAR"], 1),
        ],
    );

    let branches =
        choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]).expect("two choices");

    let branch = only_branch(&branches);
    let calendars = branch
        .join_tree
        .instances()
        .iter()
        .filter(|instance| instance.step.to == name("CALENDAR"))
        .count();
    assert_eq!(calendars, 2);
    assert_eq!(
        bound_path(branch, &QueryElement::Dimension(0)),
        [step("ORDER_DATE", "ORDERS", "CALENDAR")]
    );
    assert_eq!(
        bound_path(branch, &QueryElement::Dimension(1)),
        [
            step("SHIP_DATE", "ORDERS", "CALENDAR"),
            step("CALENDAR_YEAR", "CALENDAR", "FISCAL_YEARS")
        ]
    );
}

#[test]
fn e3_4_2_the_join_tree_holds_the_paths_to_dimension_and_filter_datasets() {
    let model = shop();
    let mut parts = parts_with_paths(&model, Vec::new());
    parts.filters = vec![filter(&["CUSTOMERS", "ORDERS"], 0)];
    let query = query(
        vec![dimension("REGIONS", "NAME", 0)],
        vec![metric_reference("REVENUE", 0)],
        parts,
    );

    let branches =
        choose_paths(&query, &model, vec![home("ORDERS", &[(0, 0)])]).expect("one path each");

    let branch = only_branch(&branches);
    assert_eq!(branch.join_tree.instances().len(), 2);
    let customers = QueryElement::Filter {
        filter: 0,
        dataset: name("CUSTOMERS"),
    };
    assert_eq!(
        bound_path(branch, &customers),
        [step("ORDERS_CUSTOMERS", "ORDERS", "CUSTOMERS")]
    );
    let orders = QueryElement::Filter {
        filter: 0,
        dataset: name("ORDERS"),
    };
    assert_eq!(branch.join_tree.bound_instance(&orders), None);
}

#[test]
fn e3_4_1_components_with_the_same_home_dataset_form_one_branch() {
    let model = shop();
    let parts = parts_with_paths(&model, Vec::new());
    let query = query(
        vec![dimension("ORDERS", "STATUS", 0)],
        vec![
            metric_reference("MIXED", 0),
            metric_reference("REVENUE", 1),
            metric_reference("REFUNDS", 2),
        ],
        parts,
    );

    let homes = component_homes(&query, &model).expect("classified metrics");

    assert_eq!(
        homes,
        [
            home("ORDERS", &[(0, 1), (1, 0)]),
            home("RETURNS", &[(0, 0), (2, 0)]),
        ]
    );
}

#[test]
fn e3_4_3_branches_are_ordered_by_home_dataset_with_their_own_paths() {
    let model = shop();
    let parts = parts_with_paths(&model, Vec::new());
    let query = query(
        vec![dimension("CUSTOMERS", "SEGMENT", 0)],
        vec![
            metric_reference("REFUNDS", 0),
            metric_reference("REVENUE", 1),
        ],
        parts,
    );
    let homes = component_homes(&query, &model).expect("classified metrics");

    let branches = choose_paths(&query, &model, homes).expect("one path each");

    let found: Vec<(&str, usize)> = branches
        .as_slice()
        .iter()
        .map(|branch| (branch.home.as_str(), branch.join_tree.instances().len()))
        .collect();
    assert_eq!(found, [("ORDERS", 1), ("RETURNS", 2)]);
}
