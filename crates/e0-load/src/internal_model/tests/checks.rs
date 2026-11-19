//! E0H-4.3 and CHK-3: the recorded integrity checks and their definition hashes.

use semantic_compiler_model::internal_model::{CheckKind, InternalModel};

use super::hashes::widened;

use super::{
    ADDITIVITY_CATALOG, ADDITIVITY_MODEL, CATALOG, MODEL, SUMMARIES_CATALOG, SUMMARIES_MODEL,
    edited, fixture, load,
};

fn identifiers(model: &InternalModel) -> Vec<String> {
    model
        .parts()
        .checks
        .iter()
        .map(|check| check.check_id.clone())
        .collect()
}

fn definition_hash(model: &InternalModel, check_id: &str) -> String {
    model
        .parts()
        .checks
        .iter()
        .find(|check| check.check_id == check_id)
        .map(|check| check.definition_hash.as_str().to_owned())
        .expect("the check is recorded")
}

#[test]
fn e0h_4_3_one_check_per_usable_relationship_sorted_by_check_id() {
    assert_eq!(
        identifiers(&fixture()),
        [
            "key_uniqueness:ORDERS_CUSTOMER",
            "orphan_keys:ORDERS_CUSTOMER"
        ]
    );
}

#[test]
fn e0h_4_3_populations_and_semi_additive_metrics_have_their_checks() {
    let model = load(ADDITIVITY_MODEL, ADDITIVITY_CATALOG);
    assert_eq!(
        identifiers(&model),
        [
            "key_uniqueness:DEBT_CUSTOMERS",
            "orphan_keys:DEBT_CUSTOMERS",
            "population:active_customers",
            "semi_additive:DEBT_BALANCE",
        ]
    );
    let kinds: Vec<CheckKind> = model
        .parts()
        .checks
        .iter()
        .map(|check| check.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            CheckKind::KeyUniqueness,
            CheckKind::OrphanKeys,
            CheckKind::Population,
            CheckKind::SemiAdditive,
        ]
    );
}

#[test]
fn chk_3_a_summary_check_carries_the_summary_definition_hash() {
    let model = load(SUMMARIES_MODEL, SUMMARIES_CATALOG);
    let check = &model.parts().checks;
    assert_eq!(check.len(), 1);
    assert_eq!(check[0].check_id, "summary:ORDERS_DAILY");
    assert_eq!(
        check[0].definition_hash,
        model.parts().summaries[0].definition_hash
    );
}

#[test]
fn chk_3_a_field_the_check_does_not_read_leaves_its_hash_unchanged() {
    let original = fixture();
    let unrelated = load(&edited("'region'", "'UPPER(region)'"), CATALOG);
    let read = load(MODEL, &widened(CATALOG, "shop.main.orders", "customer_id"));
    for check_id in [
        "key_uniqueness:ORDERS_CUSTOMER",
        "orphan_keys:ORDERS_CUSTOMER",
    ] {
        assert_eq!(
            definition_hash(&unrelated, check_id),
            definition_hash(&original, check_id)
        );
        assert_ne!(
            definition_hash(&read, check_id),
            definition_hash(&original, check_id)
        );
    }
    assert_ne!(unrelated.hash(), original.hash());
}

#[test]
fn chk_3_a_semi_additive_hash_follows_only_what_the_metric_reads() {
    let original = load(ADDITIVITY_MODEL, ADDITIVITY_CATALOG);
    let unrelated = load(
        &ADDITIVITY_MODEL.replacen(
            "'COUNT(customers.customer_id)'",
            "'MAX(customers.customer_id)'",
            1,
        ),
        ADDITIVITY_CATALOG,
    );
    let check_id = "semi_additive:DEBT_BALANCE";
    assert_eq!(
        definition_hash(&unrelated, check_id),
        definition_hash(&original, check_id)
    );
    let kinds_differ =
        ADDITIVITY_MODEL.replacen("\"on_stale\": \"error\"", "\"on_stale\": \"exclude\"", 1);
    assert_ne!(
        definition_hash(&load(&kinds_differ, ADDITIVITY_CATALOG), check_id),
        definition_hash(&original, check_id)
    );
}
