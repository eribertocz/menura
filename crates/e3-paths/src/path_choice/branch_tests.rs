//! Unit tests of chosen paths, join trees and branches (QRY-4.3, E3-4), named after the
//! rule they check.

use menura_representations::paths::QueryElement;

use super::component_homes;
use super::test_support::{
    bound_path, chosen, date_range_on, home, only_branch, parts_with_paths, revenue_by, shop, step,
};
use crate::resolve_paths;
use crate::test_support::{dimension, filter, metric_reference, name, query};

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

    let branches = resolve_paths(&query, &model).expect("one path chosen");

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

    let branches = resolve_paths(&query, &model).expect("two choices");

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

    let branches = resolve_paths(&query, &model).expect("one path each");

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
    let branches = resolve_paths(&query, &model).expect("one path each");

    let found: Vec<(&str, usize)> = branches
        .as_slice()
        .iter()
        .map(|branch| (branch.home.as_str(), branch.join_tree.instances().len()))
        .collect();
    assert_eq!(found, [("ORDERS", 1), ("RETURNS", 2)]);
}
