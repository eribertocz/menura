//! E0-4.2a, E0H-2.3: a dataset with at least one route to a dataset with `row_access` that
//! goes through an unusable relationship is unusable (`E0H-01`), even if other routes stay
//! usable, because one of its row policies could not be propagated; propagation reports no new
//! warning.

use menura_model::internal_model::{DiagnosticCode, ElementKind, InternalModel};

use super::{load, name};

const MODEL: &str =
    include_str!("../../../../../conformance/models/e0-row-access-unreachable/model.yaml");
const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-row-access-unreachable/catalog-schema.json");
const PARTIAL_MODEL: &str =
    include_str!("../../../../../conformance/models/e0-row-access-partial-route/model.yaml");
const PARTIAL_CATALOG: &str = include_str!(
    "../../../../../conformance/models/e0-row-access-partial-route/catalog-schema.json"
);

/// The relationship made unusable by `E0E-01`.
const BROKEN: &str = "from_columns: [customer_id, region]";

fn unreachable() -> InternalModel {
    load(MODEL, CATALOG)
}

/// The model with the relationship repaired, so the route is usable.
fn repaired() -> InternalModel {
    assert!(MODEL.contains(BROKEN), "the model contains `{BROKEN}`");
    load(
        &MODEL.replacen(BROKEN, "from_columns: [customer_id]", 1),
        CATALOG,
    )
}

/// The model with a second, usable relationship from `orders` to `customers`.
fn with_usable_route() -> InternalModel {
    let second = "    metrics:\n";
    let route = "      - name: orders_customers_by_id\n        from: orders\n        to: customers\n        \
                 from_columns: [customer_id]\n        to_columns: [customer_id]\n";
    assert!(
        MODEL.contains(second),
        "the model has semantic-model metrics"
    );
    load(
        &MODEL.replacen(second, &format!("{route}{second}"), 1),
        CATALOG,
    )
}

fn e0h_01() -> Vec<DiagnosticCode> {
    vec![DiagnosticCode::new("E0H-01")]
}

fn dataset_errors(model: &InternalModel, dataset: &str) -> Vec<DiagnosticCode> {
    model
        .dataset(&name(dataset))
        .expect("the dataset")
        .query_errors
        .clone()
}

#[test]
fn e0_4_2a_a_dataset_whose_every_route_to_row_access_is_unusable_is_unusable() {
    let model = unreachable();

    assert_eq!(dataset_errors(&model, "orders"), e0h_01());
    let unusable = model
        .parts()
        .unusable
        .iter()
        .find(|element| element.kind == ElementKind::Dataset && element.name == [name("orders")])
        .expect("orders is unusable");
    assert_eq!(unusable.query_errors, e0h_01());
    assert_eq!(unusable.load_warning, None);
}

#[test]
fn e0_4_2a_e0h_01_propagates_to_the_fields_and_metrics_that_read_the_dataset() {
    let model = unreachable();

    let amount = model
        .dataset(&name("orders"))
        .and_then(|dataset| dataset.field(&name("amount")))
        .expect("orders.amount");
    assert_eq!(amount.query_errors, e0h_01());
    let total = model.metric(&name("order_total")).expect("order_total");
    assert_eq!(total.query_errors, e0h_01());
}

#[test]
fn e0_4_2a_the_dataset_with_row_access_stays_usable() {
    assert_eq!(dataset_errors(&unreachable(), "customers"), Vec::new());
}

#[test]
fn e0_4_2a_a_repaired_route_keeps_the_dataset_usable() {
    assert_eq!(dataset_errors(&repaired(), "orders"), Vec::new());
}

#[test]
fn e0_4_2a_a_second_usable_route_to_the_same_dataset_does_not_restore_it() {
    assert_eq!(dataset_errors(&with_usable_route(), "orders"), e0h_01());
}

#[test]
fn e0_4_2a_a_usable_route_to_another_policy_does_not_hide_the_unusable_one() {
    let model = load(PARTIAL_MODEL, PARTIAL_CATALOG);

    assert_eq!(dataset_errors(&model, "orders"), e0h_01());
    assert_eq!(dataset_errors(&model, "stores"), Vec::new());
    assert_eq!(dataset_errors(&model, "suppliers"), Vec::new());
}

#[test]
fn e0_4_2a_e0h_01_adds_no_warning() {
    let codes: Vec<String> = unreachable()
        .parts()
        .warnings
        .iter()
        .map(|warning| warning.code.as_str().to_owned())
        .collect();
    assert_eq!(codes, ["W0E-04"]);
}
