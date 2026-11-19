//! E0H-1.4: collections are ordered by normalized name, positional lists keep their order, and
//! unusable elements are ordered by kind, then by qualified name.

use semantic_compiler_model::internal_model::{ElementKind, NormalizedName};

use super::{fixture, name};

fn names<Element>(
    elements: &[Element],
    name_of: impl Fn(&Element) -> &NormalizedName,
) -> Vec<String> {
    elements
        .iter()
        .map(|element| name_of(element).as_str().to_owned())
        .collect()
}

#[test]
fn e0h_1_4_named_collections_are_ordered_by_normalized_name() {
    let model = fixture();
    let parts = model.parts();
    assert_eq!(
        names(&parts.datasets, |dataset| &dataset.name),
        ["CUSTOMERS", "ORDERS"]
    );
    let orders = model.dataset(&name("orders")).expect("orders");
    assert_eq!(
        names(&orders.fields, |field| &field.name),
        ["AMOUNT", "CUSTOMER_ID", "DISCOUNT", "ORDER_ID", "STATUS"]
    );
    assert_eq!(
        names(&parts.metrics, |metric| &metric.name),
        ["NET_AMOUNT", "TOTAL_AMOUNT", "TOTAL_DISCOUNT"]
    );
    assert_eq!(
        names(&parts.relationships, |relationship| &relationship.name),
        ["ORDERS_CUSTOMER", "ORDERS_STATUS_CUSTOMER"]
    );
}

#[test]
fn e0h_1_4_unusable_elements_are_ordered_by_kind_then_qualified_name() {
    let model = fixture();
    let listed: Vec<(ElementKind, Vec<String>)> = model
        .parts()
        .unusable
        .iter()
        .map(|element| {
            (
                element.kind,
                element
                    .name
                    .iter()
                    .map(|part| part.as_str().to_owned())
                    .collect(),
            )
        })
        .collect();
    let qualified = |parts: &[&str]| parts.iter().map(|part| (*part).to_owned()).collect();
    assert_eq!(
        listed,
        vec![
            (ElementKind::Field, qualified(&["ORDERS", "DISCOUNT"])),
            (
                ElementKind::Relationship,
                qualified(&["ORDERS_STATUS_CUSTOMER"])
            ),
            (ElementKind::Metric, qualified(&["NET_AMOUNT"])),
            (ElementKind::Metric, qualified(&["TOTAL_DISCOUNT"])),
        ]
    );
}

#[test]
fn e0h_1_4_positional_lists_keep_their_order() {
    let model = fixture();
    let relationship = model
        .relationship(&name("orders_status_customer"))
        .expect("the relationship");
    assert_eq!(relationship.from_columns, vec![name("status")]);
    assert_eq!(relationship.to_columns, vec![name("customer_id")]);
    let net = model.metric(&name("net_amount")).expect("net_amount");
    let tree = net.expression.as_ref().expect("the tree").json_form();
    let text = format!("{tree:?}");
    let amount = text.find("AMOUNT").expect("amount is read");
    let discount = text.find("DISCOUNT").expect("discount is read");
    assert!(amount < discount, "{text}");
}
