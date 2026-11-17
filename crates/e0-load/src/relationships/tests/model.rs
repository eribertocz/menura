//! The analysis of the `e0-relationships` conformance model and of the dispositions of E0-4.

use super::super::RelationshipCode;
use super::super::output::PathCount;
use super::{
    REFERENCES_MISSING, RELATIONSHIPS, analyze, dataset, dataset_names, index_of, relationship,
    summary,
};

#[test]
fn e0e_2_1_every_dataset_except_summaries_is_a_node() {
    let relationships = analyze(RELATIONSHIPS);
    assert_eq!(
        dataset_names(&relationships),
        [
            "ORDERS",
            "CUSTOMERS",
            "CUSTOMER_PROFILES",
            "STORES",
            "REGIONS",
            "CALENDAR",
            "PROMOTIONS",
            "EMPLOYEES",
            "EXCHANGE_RATES",
        ]
    );
}

#[test]
fn e0e_2_2_one_named_edge_per_usable_relationship_from_many_to_one() {
    let relationships = analyze(RELATIONSHIPS);
    let names = dataset_names(&relationships);
    let edges: Vec<(&str, &str, &str)> = relationships
        .graph
        .edges
        .iter()
        .map(|edge| (edge.name.as_str(), names[edge.from.0], names[edge.to.0]))
        .collect();
    assert_eq!(
        edges,
        [
            ("ORDERS_CUSTOMERS", "ORDERS", "CUSTOMERS"),
            ("ORDERS_STORES", "ORDERS", "STORES"),
            ("CUSTOMERS_REGIONS", "CUSTOMERS", "REGIONS"),
            ("STORES_REGIONS", "STORES", "REGIONS"),
            ("ORDERS_ORDER_DATE", "ORDERS", "CALENDAR"),
            ("ORDERS_SHIP_DATE", "ORDERS", "CALENDAR"),
            ("ORDERS_PROMOTIONS", "ORDERS", "PROMOTIONS"),
            ("CUSTOMERS_PROFILES", "CUSTOMERS", "CUSTOMER_PROFILES"),
            ("EMPLOYEES_STORES", "EMPLOYEES", "STORES"),
            ("STORES_MANAGERS", "STORES", "EMPLOYEES"),
        ]
    );
    assert_eq!(
        relationships.graph.edges[0].pointer.to_string(),
        relationship(0)
    );
}

#[test]
fn e0_4_3_every_warning_in_document_order_then_by_code() {
    let relationships = analyze(RELATIONSHIPS);
    assert_eq!(
        summary(&relationships.warnings),
        [
            ("W0E-01", "W0E-01", dataset(0)),
            ("W0E-01", "W0E-01", dataset(0)),
            (
                "W0E-02",
                "W0E-02",
                format!("{}/to_columns", relationship(6))
            ),
            ("W0E-03", "W0E-03", relationship(10)),
            ("W0E-04", "E0E-01", relationship(11)),
            ("W0E-04", "E0E-02", format!("{}/from", relationship(12))),
        ]
    );
}

#[test]
fn e0e_3_1_and_3_6_invalid_relationships_are_unusable_and_excluded() {
    let relationships = analyze(RELATIONSHIPS);
    let unusable: Vec<String> = relationships
        .unusable
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(unusable, [relationship(11), relationship(12)]);
    assert!(
        relationships
            .graph
            .edges
            .iter()
            .all(|edge| !unusable.contains(&edge.pointer.to_string()))
    );
    assert_eq!(
        RelationshipCode::INVALID_RELATIONSHIP_QUERY_ERROR,
        "E_NAME_NOT_FOUND"
    );
}

#[test]
fn e0e_3_4_a_relationship_from_a_key_is_recorded_as_one_to_one() {
    let relationships = analyze(RELATIONSHIPS);
    let one_to_one: Vec<&str> = relationships
        .graph
        .edges
        .iter()
        .filter(|edge| edge.one_to_one)
        .map(|edge| edge.name.as_str())
        .collect();
    assert_eq!(one_to_one, ["CUSTOMERS_PROFILES"]);
}

#[test]
fn e0e_4_2_a_diamond_and_two_relationships_report_each_pair_once() {
    let relationships = analyze(RELATIONSHIPS);
    let pairs: Vec<(usize, usize)> = relationships
        .several_paths
        .iter()
        .map(|pair| (pair.from.0, pair.to.0))
        .collect();
    let orders = index_of(&relationships, "ORDERS");
    assert_eq!(
        pairs,
        [
            (orders, index_of(&relationships, "REGIONS")),
            (orders, index_of(&relationships, "CALENDAR")),
        ]
    );
    let messages: Vec<&str> = relationships
        .warnings
        .iter()
        .filter(|warning| warning.code == RelationshipCode::SeveralPaths)
        .map(|warning| warning.message.as_str())
        .collect();
    assert!(messages[0].contains("`regions`"));
    assert!(messages[1].contains("`calendar`"));
}

#[test]
fn e0e_2_4_and_3_5_a_cycle_loads_and_its_paths_stay_finite() {
    let relationships = analyze(RELATIONSHIPS);
    let stores = index_of(&relationships, "STORES");
    let employees = index_of(&relationships, "EMPLOYEES");
    let reached = |from: usize| -> Vec<(usize, PathCount)> {
        relationships.reachable[from]
            .iter()
            .map(|reached| (reached.dataset.0, reached.paths))
            .collect()
    };
    let regions = index_of(&relationships, "REGIONS");
    assert_eq!(
        reached(stores),
        [(regions, PathCount::One), (employees, PathCount::One)]
    );
    assert_eq!(
        reached(employees),
        [(stores, PathCount::One), (regions, PathCount::One)]
    );
}

#[test]
fn e0e_5_1_a_dataset_without_relationships_is_allowed() {
    let relationships = analyze(RELATIONSHIPS);
    let exchange_rates = index_of(&relationships, "EXCHANGE_RATES");
    assert!(relationships.reachable[exchange_rates].is_empty());
    assert!(
        relationships
            .reachable
            .iter()
            .flatten()
            .all(|reached| reached.dataset.0 != exchange_rates)
    );
}

#[test]
fn e0_4_2_relationships_already_unusable_are_excluded_without_a_warning() {
    let relationships = analyze(REFERENCES_MISSING);
    let edges: Vec<&str> = relationships
        .graph
        .edges
        .iter()
        .map(|edge| edge.name.as_str())
        .collect();
    assert_eq!(edges, ["ORDERS_CUSTOMERS"]);
    assert!(relationships.warnings.is_empty());
    assert!(relationships.unusable.is_empty());
}
