//! E0-4.2a: unusability propagates, transitively, with every query error code, and no new
//! warning.

use menura_model::internal_model::{DiagnosticCode, ElementKind, InternalModel};

use super::{CATALOG, edited, fixture, load, name};

fn codes(texts: &[&str]) -> Vec<DiagnosticCode> {
    texts.iter().map(|text| DiagnosticCode::new(text)).collect()
}

fn unusable_warning(model: &InternalModel, kind: ElementKind, last: &str) -> Option<String> {
    model
        .parts()
        .unusable
        .iter()
        .find(|element| element.kind == kind && element.name.last() == Some(&name(last)))
        .and_then(|element| element.load_warning.as_ref())
        .map(|code| code.as_str().to_owned())
}

#[test]
fn e0_4_2a_a_metric_reading_an_unusable_field_is_unusable_with_its_code() {
    let model = fixture();
    let metric = model
        .metric(&name("total_discount"))
        .expect("total_discount");
    assert_eq!(metric.query_errors, codes(&["E0D-03"]));
    assert_eq!(
        unusable_warning(&model, ElementKind::Metric, "total_discount"),
        None
    );
}

#[test]
fn e0_4_2a_an_element_holds_its_own_code_and_every_propagated_one() {
    let model = fixture();
    let metric = model.metric(&name("net_amount")).expect("net_amount");
    assert_eq!(metric.query_errors, codes(&["E0D-03", "E0D-04"]));
    assert_eq!(
        unusable_warning(&model, ElementKind::Metric, "net_amount").as_deref(),
        Some("W0D-01")
    );
}

#[test]
fn e0_4_2a_a_relationship_naming_an_unusable_field_is_unusable_and_out_of_the_graph() {
    let model = load(
        &edited(
            "        from_columns: [customer_id]\n",
            "        from_columns: [discount]\n",
        ),
        CATALOG,
    );
    let relationship = model
        .relationship(&name("orders_customer"))
        .expect("the relationship");
    assert!(
        relationship
            .query_errors
            .contains(&DiagnosticCode::new("E0D-03"))
    );
    assert!(model.parts().graph.edges.is_empty());
    assert!(
        model
            .parts()
            .graph
            .reachability
            .iter()
            .all(|reach| reach.reached.is_empty())
    );
}

#[test]
fn e0_4_2a_an_unusable_dataset_makes_its_fields_and_relationships_unusable() {
    let model = load(
        &edited("primary_key: [customer_id]", "primary_key: [nobody]"),
        CATALOG,
    );
    let customers = model.dataset(&name("customers")).expect("customers");
    assert_eq!(customers.query_errors, codes(&["E0D-06"]));
    for field in &customers.fields {
        assert_eq!(field.query_errors, codes(&["E0D-06"]), "{:?}", field.name);
    }
    let relationship = model
        .relationship(&name("orders_customer"))
        .expect("the relationship");
    assert_eq!(relationship.query_errors, codes(&["E0D-06"]));
    assert_eq!(unusable_warning(&model, ElementKind::Field, "region"), None);
}
