//! E0H-1.1, E0H-1.2, E0H-1.5, E0H-2 and E0H-2.3: what the internal model holds.

use semantic_compiler_contracts::types::common::{IntegerBits, LogicalType};
use semantic_compiler_model::internal_model::{
    AccessMode, Additivity, DatasetPair, DiagnosticCode, GraphEdge, PathCount, ReachedDataset,
    RoleName, TypedValue, ValueType,
};

use super::{CATALOG, MODEL, edited, fixture, load, name};

#[test]
fn e0h_1_1_the_same_input_builds_equal_models() {
    assert_eq!(fixture(), fixture());
}

#[test]
fn e0h_2_datasets_hold_source_typed_fields_keys_and_access() {
    let model = fixture();
    let orders = model.dataset(&name("orders")).expect("orders");
    assert_eq!(orders.source, "shop.main.orders");
    assert_eq!(orders.primary_key, vec![name("order_id")]);
    assert!(!orders.summary);
    let amount = orders.field(&name("amount")).expect("amount");
    assert_eq!(
        amount.typed,
        Some(TypedValue {
            value_type: ValueType::Logical(LogicalType::Integer {
                bits: IntegerBits::ThirtyTwo,
                signed: true,
            }),
            nullable: false,
        })
    );
    assert_eq!(
        amount.descriptions.description.as_deref(),
        Some("Order amount")
    );
    let access = orders.access.as_ref().expect("orders has access");
    assert_eq!(
        access.mode,
        AccessMode::Roles(vec![RoleName::new("analyst")])
    );
}

#[test]
fn e0h_2_metrics_security_and_warnings_are_in_the_model() {
    let model = fixture();
    let parts = model.parts();
    let total = model.metric(&name("total_amount")).expect("total_amount");
    let classification = total.classification.as_ref().expect("classified");
    assert_eq!(classification.additivity, Additivity::Additive);
    assert_eq!(classification.components.len(), 1);
    assert_eq!(classification.home_datasets, vec![name("orders")]);
    assert!(total.expression.is_some() && total.typed.is_some());
    let security = parts.security_context.as_ref().expect("security context");
    assert_eq!(security.roles, vec![RoleName::new("analyst")]);
    assert!(parts.descriptions.description.is_some());
    let codes: Vec<&str> = parts
        .warnings
        .iter()
        .map(|warning| warning.code.as_str())
        .collect();
    assert!(codes.contains(&"W0D-01") && codes.contains(&"W0F-01") && codes.contains(&"W0E-01"));
}

#[test]
fn e0h_1_5_every_element_keeps_its_source_location() {
    let model = fixture();
    let orders = model.dataset(&name("orders")).expect("orders");
    let discount = orders.field(&name("discount")).expect("discount");
    assert_eq!(
        discount.location.pointer,
        "/semantic_model/0/datasets/0/fields/4"
    );
    assert!(discount.location.line > orders.location.line);
    let metric = model.metric(&name("net_amount")).expect("net_amount");
    assert_eq!(metric.location.pointer, "/semantic_model/0/metrics/2");
}

#[test]
fn e0h_2_3_the_graph_excludes_a_relationship_made_unusable_after_e0e() {
    let model = fixture();
    let parts = model.parts();
    let unusable = model
        .relationship(&name("orders_status_customer"))
        .expect("the relationship is in the model");
    assert_eq!(unusable.query_errors, vec![DiagnosticCode::new("E0F-05")]);
    assert_eq!(
        parts.graph.edges,
        vec![GraphEdge {
            relationship: name("orders_customer"),
            from: name("orders"),
            to: name("customers"),
            one_to_one: false,
        }]
    );
    let orders = parts
        .graph
        .reachability
        .iter()
        .find(|reach| reach.dataset == name("orders"))
        .expect("orders is a node");
    assert_eq!(
        orders.reached,
        vec![ReachedDataset {
            dataset: name("customers"),
            paths: PathCount::One,
        }]
    );
    assert_eq!(parts.graph.several_paths, Vec::<DatasetPair>::new());
}

#[test]
fn e0h_2_3_with_both_relationships_usable_there_are_several_paths() {
    let model = load(
        &edited("from_columns: [status]", "from_columns: [order_id]"),
        CATALOG,
    );
    assert_eq!(
        model.parts().graph.several_paths,
        vec![DatasetPair {
            from: name("orders"),
            to: name("customers"),
        }]
    );
}
