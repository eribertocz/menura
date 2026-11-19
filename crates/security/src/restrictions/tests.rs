//! SCY-3.1, SCY-3.1a and SCY-5.1.8.

use std::collections::{BTreeMap, BTreeSet};

use super::{RestrictionInput, restrictions};
use crate::role::Role;

fn roles(names: &[&str]) -> BTreeSet<Role> {
    names.iter().map(|name| Role::new(name)).collect()
}

fn set<'item>(items: &[&'item str]) -> BTreeSet<&'item str> {
    items.iter().copied().collect()
}

/// `unit_cost` and `cost_copy` read `cost`, `margin` reads `price` and `cost`, `price` reads
/// `price`, `label` reads no column.
fn input() -> RestrictionInput<&'static str, &'static str, &'static str> {
    RestrictionInput {
        field_columns: BTreeMap::from([
            ("unit_cost", set(&["cost"])),
            ("cost_copy", set(&["cost"])),
            ("margin", set(&["price", "cost"])),
            ("price", set(&["price"])),
            ("label", set(&[])),
        ]),
        declared: Vec::new(),
        metric_fields: BTreeMap::new(),
        metric_inputs: BTreeMap::new(),
    }
}

#[test]
fn scy_5_1_8_nothing_is_restricted_without_fields() {
    let found = restrictions(&input());
    assert!(found.fields.is_empty() && found.metrics.is_empty());
}

#[test]
fn scy_3_1a_fields_over_the_same_column_get_the_same_restriction() {
    let mut input = input();
    input.declared = vec![("unit_cost", roles(&["admin"]))];
    let found = restrictions(&input);
    assert_eq!(found.fields.get("unit_cost"), Some(&roles(&["admin"])));
    assert_eq!(found.fields.get("cost_copy"), Some(&roles(&["admin"])));
    assert_eq!(found.fields.get("margin"), Some(&roles(&["admin"])));
    assert_eq!(found.fields.get("price"), None);
    assert_eq!(found.fields.get("label"), None);
}

#[test]
fn scy_3_1a_the_columns_protected_are_those_of_the_named_field() {
    let mut input = input();
    input.declared = vec![("margin", roles(&["admin"]))];
    let found = restrictions(&input);
    for field in ["margin", "unit_cost", "cost_copy", "price"] {
        assert_eq!(found.fields.get(field), Some(&roles(&["admin"])), "{field}");
    }
    assert_eq!(found.fields.get("label"), None);
}

#[test]
fn scy_3_1a_several_restrictions_intersect() {
    let mut input = input();
    input.declared = vec![
        ("unit_cost", roles(&["admin", "finance"])),
        ("price", roles(&["admin", "sales"])),
    ];
    let found = restrictions(&input);
    assert_eq!(found.fields.get("margin"), Some(&roles(&["admin"])));
    assert_eq!(
        found.fields.get("unit_cost"),
        Some(&roles(&["admin", "finance"]))
    );
    assert_eq!(found.fields.get("price"), Some(&roles(&["admin", "sales"])));
}

#[test]
fn scy_3_1_a_field_without_columns_is_restricted_by_its_own_item() {
    let mut input = input();
    input.declared = vec![("label", roles(&["admin"]))];
    let found = restrictions(&input);
    assert_eq!(found.fields.get("label"), Some(&roles(&["admin"])));
    assert_eq!(found.fields.len(), 1);
}

#[test]
fn scy_3_1_metrics_take_the_restrictions_of_the_fields_they_read() {
    let mut input = input();
    input.declared = vec![
        ("unit_cost", roles(&["admin", "finance"])),
        ("price", roles(&["admin", "sales"])),
    ];
    input.metric_fields = BTreeMap::from([
        ("total_cost", set(&["cost_copy"])),
        ("total_margin", set(&["unit_cost", "price"])),
        ("labels", set(&["label"])),
    ]);
    input.metric_inputs = BTreeMap::from([
        ("cost_share", set(&["total_cost", "labels"])),
        ("mixed", set(&["total_cost", "total_margin"])),
        ("open", set(&["labels"])),
    ]);
    let found = restrictions(&input);
    assert_eq!(
        found.metrics.get("total_cost"),
        Some(&roles(&["admin", "finance"]))
    );
    assert_eq!(found.metrics.get("total_margin"), Some(&roles(&["admin"])));
    assert_eq!(found.metrics.get("labels"), None);
    assert_eq!(
        found.metrics.get("cost_share"),
        Some(&roles(&["admin", "finance"]))
    );
    assert_eq!(found.metrics.get("mixed"), Some(&roles(&["admin"])));
    assert_eq!(found.metrics.get("open"), None);
}

#[test]
fn scy_5_1_8_a_cycle_ends() {
    let mut input = input();
    input.declared = vec![("unit_cost", roles(&["admin"]))];
    input.metric_fields = BTreeMap::from([("base", set(&["unit_cost"]))]);
    input.metric_inputs = BTreeMap::from([("a", set(&["b", "base"])), ("b", set(&["a"]))]);
    let found = restrictions(&input);
    assert_eq!(found.metrics.get("a"), Some(&roles(&["admin"])));
}
