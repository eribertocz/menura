//! E0H-4.3 and CHK-3: the recorded integrity checks and their definition hashes.

use menura_model::internal_model::{CheckKind, InternalModel};

use super::hashes::widened;

use menura_contracts::types::audit_record::OssieFormat;

use super::{
    ADDITIVITY_CATALOG, ADDITIVITY_MODEL, CATALOG, DEVELOPMENT_CATALOG, DEVELOPMENT_MODEL, MODEL,
    SUMMARIES_CATALOG, SUMMARIES_MODEL, edited, fixture, load, load_in_format,
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
            "population:ACTIVE_CUSTOMERS",
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
    let read = load(MODEL, &widened(CATALOG, "shop.orders", "customer_id"));
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

#[test]
fn e0h_4_3_a_population_target_is_its_normalized_name() {
    let quoted = ADDITIVITY_MODEL.replacen(
        "\"name\": \"active_customers\"",
        "\"name\": \"\\\"ACTIVE_CUSTOMERS\\\"\"",
        1,
    );
    assert_ne!(quoted, ADDITIVITY_MODEL);
    let check_id = "population:ACTIVE_CUSTOMERS";
    let original = load(ADDITIVITY_MODEL, ADDITIVITY_CATALOG);
    let model = load(&quoted, ADDITIVITY_CATALOG);
    assert!(identifiers(&original).contains(&check_id.to_owned()));
    assert_eq!(identifiers(&model), identifiers(&original));
    assert_eq!(
        definition_hash(&model, check_id),
        definition_hash(&original, check_id)
    );
    let check = model
        .parts()
        .checks
        .iter()
        .find(|check| check.check_id == check_id)
        .expect("the check is recorded");
    assert_eq!(check.target, "ACTIVE_CUSTOMERS");
}

#[test]
fn e0h_4_3_a_semi_additive_dataset_metric_is_qualified_by_its_dataset() {
    let start = DEVELOPMENT_MODEL
        .find("    metrics:\n      - name: customer_snapshot")
        .expect("the customers metric");
    let end = DEVELOPMENT_MODEL[start..]
        .find("relationships:")
        .map(|offset| start + offset)
        .expect("the relationships");
    let model_text = format!(
        "{}{}",
        &DEVELOPMENT_MODEL[..start],
        &DEVELOPMENT_MODEL[end..]
    );
    let model = load_in_format(&model_text, DEVELOPMENT_CATALOG, OssieFormat::Development);
    let check = model
        .parts()
        .checks
        .iter()
        .find(|check| check.kind == CheckKind::SemiAdditive)
        .expect("the dataset metric has its check");
    assert_eq!(check.check_id, "semi_additive:DEBT.DEBT_BALANCE");
    assert_eq!(check.target, "DEBT.DEBT_BALANCE");
}
