//! Unit tests of the branches (E3-4, E3-5).

use super::{
    Branch, Branches, ComponentKey, ElementPath, InstanceId, JoinStep, JoinTree, QueryElement,
};
use crate::test_support::name;

fn step(relationship: &str, from: &str, to: &str) -> JoinStep {
    JoinStep {
        relationship: name(relationship),
        from: name(from),
        to: name(to),
    }
}

fn branch(home: &str, metric: usize) -> Branch {
    Branch {
        home: name(home),
        components: vec![ComponentKey {
            metric,
            component: 0,
        }],
        join_tree: JoinTree::default(),
    }
}

#[test]
fn e3_4_1_components_with_the_same_home_dataset_form_one_branch() {
    assert!(Branches::new(vec![branch("ORDERS", 0), branch("ORDERS", 1)]).is_none());

    let mut shared = branch("ORDERS", 0);
    shared.components.push(ComponentKey {
        metric: 1,
        component: 0,
    });
    let branches = Branches::new(vec![shared]).expect("one home per branch");
    let only = branches.as_slice().first().expect("one branch");
    assert_eq!(only.components.len(), 2);
}

fn path(element: QueryElement, steps: &[&JoinStep]) -> ElementPath {
    ElementPath {
        element,
        steps: steps.iter().map(|step| (*step).clone()).collect(),
    }
}

fn steps_of(tree: &JoinTree) -> Vec<&JoinStep> {
    tree.instances()
        .iter()
        .map(|instance| &instance.step)
        .collect()
}

#[test]
fn e3_4_2_a_join_tree_is_the_union_of_the_paths() {
    let to_customers = step("ORDERS_TO_CUSTOMERS", "ORDERS", "CUSTOMERS");
    let to_regions = step("CUSTOMERS_TO_REGIONS", "CUSTOMERS", "REGIONS");
    let tree = JoinTree::from_paths(vec![
        path(QueryElement::Dimension(0), &[&to_customers]),
        path(QueryElement::Dimension(1), &[&to_customers, &to_regions]),
    ]);

    assert_eq!(steps_of(&tree), [&to_customers, &to_regions]);
    let regions = tree.instances().get(1).expect("two instances");
    assert_eq!(regions.parent.map(InstanceId::index), Some(0));
}

#[test]
fn e3_4_2_a_join_tree_lists_instances_from_the_home_dataset_outward() {
    let to_products = step("ORDERS_TO_PRODUCTS", "ORDERS", "PRODUCTS");
    let to_customers = step("ORDERS_TO_CUSTOMERS", "ORDERS", "CUSTOMERS");
    let to_brands = step("PRODUCTS_TO_BRANDS", "PRODUCTS", "BRANDS");
    let to_regions = step("CUSTOMERS_TO_REGIONS", "CUSTOMERS", "REGIONS");
    let tree = JoinTree::from_paths(vec![
        path(QueryElement::Dimension(0), &[&to_products, &to_brands]),
        path(QueryElement::Dimension(1), &[&to_customers, &to_regions]),
    ]);

    assert_eq!(
        steps_of(&tree),
        [&to_customers, &to_products, &to_regions, &to_brands]
    );
}

#[test]
fn e3_4_2_a_two_step_sequences_to_one_dataset_are_two_instances() {
    let shipped = step("SHIP_DATE", "ORDERS", "CALENDAR");
    let ordered = step("ORDER_DATE", "ORDERS", "CALENDAR");
    let tree = JoinTree::from_paths(vec![
        path(QueryElement::Dimension(0), &[&shipped]),
        path(QueryElement::Dimension(1), &[&ordered]),
        path(QueryElement::DateRange, &[&ordered]),
    ]);

    assert_eq!(steps_of(&tree), [&ordered, &shipped]);
    let bound = |element| tree.bound_instance(&element).map(InstanceId::index);
    assert_eq!(bound(QueryElement::Dimension(0)), Some(1));
    assert_eq!(bound(QueryElement::Dimension(1)), Some(0));
    assert_eq!(bound(QueryElement::DateRange), Some(0));
}

#[test]
fn e3_4_2_a_the_same_step_after_two_prefixes_is_two_instances() {
    let direct = step("ORDERS_TO_STORES", "ORDERS", "STORES");
    let through_staff = step("ORDERS_TO_STAFF", "ORDERS", "STAFF");
    let staff_store = step("STAFF_TO_STORES", "STAFF", "STORES");
    let store_city = step("STORES_TO_CITIES", "STORES", "CITIES");
    let tree = JoinTree::from_paths(vec![
        path(QueryElement::Dimension(0), &[&direct, &store_city]),
        path(
            QueryElement::Dimension(1),
            &[&through_staff, &staff_store, &store_city],
        ),
    ]);

    assert_eq!(tree.instances().len(), 5);
    let second_city = tree
        .bound_instance(&QueryElement::Dimension(1))
        .expect("bound");
    assert_eq!(
        tree.path_to(second_city),
        [&through_staff, &staff_store, &store_city]
    );
    let first_city = tree
        .bound_instance(&QueryElement::Dimension(0))
        .expect("bound");
    assert_eq!(tree.path_to(first_city), [&direct, &store_city]);
}

#[test]
fn e3_4_2_a_an_element_of_the_home_dataset_is_bound_to_no_instance() {
    let filter = QueryElement::Filter {
        filter: 0,
        dataset: name("ORDERS"),
    };
    let tree = JoinTree::from_paths(vec![path(filter.clone(), &[])]);

    assert!(tree.is_empty());
    assert_eq!(tree.bound_instance(&filter), None);
}

#[test]
fn e3_4_3_branches_sort_by_the_normalized_name_of_their_home_dataset() {
    let branches = Branches::new(vec![
        branch("RETURNS", 0),
        branch("ORDERS", 1),
        branch("PAYMENTS", 2),
    ])
    .expect("different homes");

    let homes: Vec<&str> = branches
        .as_slice()
        .iter()
        .map(|branch| branch.home.as_str())
        .collect();
    assert_eq!(homes, ["ORDERS", "PAYMENTS", "RETURNS"]);
}

#[test]
fn e3_5_3_a_dimension_only_query_is_one_branch_without_components() {
    let to_customers = step("ORDERS_TO_CUSTOMERS", "ORDERS", "CUSTOMERS");
    let tree = JoinTree::from_paths(vec![path(QueryElement::Dimension(0), &[&to_customers])]);
    let branches = Branches::dimension_only(name("ORDERS"), tree.clone());

    let only = branches.as_slice();
    assert_eq!(only.len(), 1);
    let anchor = only.first().expect("one branch");
    assert_eq!(anchor.home, name("ORDERS"));
    assert!(anchor.components.is_empty());
    assert_eq!(anchor.join_tree, tree);
}
