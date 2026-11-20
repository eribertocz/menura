//! Unit tests of the branches (E3-4, E3-5).

use super::{Branch, Branches, ComponentKey, JoinStep, JoinTree};
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

#[test]
fn e3_4_2_a_join_tree_is_the_union_of_the_paths() {
    let to_customers = step("ORDERS_TO_CUSTOMERS", "ORDERS", "CUSTOMERS");
    let to_regions = step("CUSTOMERS_TO_REGIONS", "CUSTOMERS", "REGIONS");
    let tree = JoinTree::from_paths(vec![
        vec![to_customers.clone()],
        vec![to_customers.clone(), to_regions.clone()],
    ]);

    assert_eq!(tree.steps(), [to_customers, to_regions]);
}

#[test]
fn e3_4_2_a_join_tree_lists_steps_from_the_home_dataset_outward() {
    let to_products = step("ORDERS_TO_PRODUCTS", "ORDERS", "PRODUCTS");
    let to_customers = step("ORDERS_TO_CUSTOMERS", "ORDERS", "CUSTOMERS");
    let to_brands = step("PRODUCTS_TO_BRANDS", "PRODUCTS", "BRANDS");
    let to_regions = step("CUSTOMERS_TO_REGIONS", "CUSTOMERS", "REGIONS");
    let tree = JoinTree::from_paths(vec![
        vec![to_products.clone(), to_brands.clone()],
        vec![to_customers.clone(), to_regions.clone()],
    ]);

    assert_eq!(
        tree.steps(),
        [to_customers, to_products, to_regions, to_brands]
    );
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
    let tree = JoinTree::from_paths(vec![vec![step(
        "ORDERS_TO_CUSTOMERS",
        "ORDERS",
        "CUSTOMERS",
    )]]);
    let branches = Branches::dimension_only(name("ORDERS"), tree.clone());

    let only = branches.as_slice();
    assert_eq!(only.len(), 1);
    let anchor = only.first().expect("one branch");
    assert_eq!(anchor.home, name("ORDERS"));
    assert!(anchor.components.is_empty());
    assert_eq!(anchor.join_tree, tree);
}
