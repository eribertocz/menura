//! The additivity declaration of a classified metric: ADD-2.1, ADD-2.2, ADD-5.1.8, E0G-4.1 to
//! E0G-4.3 and `E0G-01`.

use super::super::diagnostic::ClassificationCode;
use super::super::output::Additivity;
use super::{Probe, classify_in_format, pair, probe, summary};
use crate::extensions::additivity::DeclaredAdditivity;
use crate::extensions::additivity::fixture::{
    CATALOG, DEBT_BALANCE, MODEL, STRICT_BALANCE, conformance_model, with_debt_balance_expression,
};
use menura_contracts::types::audit_record::OssieFormat;

use super::super::Classification;

fn classify(model: &str) -> Classification {
    classify_in_format(model, CATALOG, OssieFormat::Release)
}

/// The additivity of the classified metric at `pointer` of `model`, with its declaration.
fn additivity(model: &str, pointer: &str) -> (Additivity, Option<DeclaredAdditivity>) {
    match probe(&classify(model), pointer) {
        Probe::Classified(metric) => (metric.additivity, metric.declaration),
        other => panic!("{pointer} must be classified, found {other:?}"),
    }
}

/// What classification finds for `debt_balance`, declared as in ADD-8, with `expression`.
fn debt_balance(expression: &str) -> Probe {
    probe(
        &classify(&with_debt_balance_expression(expression)),
        DEBT_BALANCE,
    )
}

#[test]
fn add_8_e0g_4_3_the_example_is_semi_additive() {
    let classification = classify(MODEL);
    assert!(classification.warnings.is_empty());
    let (found, declaration) = additivity(MODEL, DEBT_BALANCE);
    assert_eq!(found, Additivity::SemiAdditive);
    assert!(matches!(
        declaration,
        Some(DeclaredAdditivity::SemiAdditive(_))
    ));
}

#[test]
fn add_2_2_e0g_4_1_declaring_non_additive_restricts_an_additive_metric() {
    assert_eq!(
        additivity(MODEL, STRICT_BALANCE),
        (
            Additivity::NonAdditive,
            Some(DeclaredAdditivity::NonAdditive)
        )
    );
}

#[test]
fn add_2_1_e0g_4_3_without_a_declaration_additivity_is_deduced() {
    assert_eq!(
        additivity(MODEL, "/semantic_model/0/metrics/2"),
        (Additivity::Additive, None)
    );
}

#[test]
fn e0g_4_1_e0g_01_semi_additive_relaxes_a_non_additive_metric() {
    let model = conformance_model("e0-additivity-conflicts");
    let classification = classify(model);
    assert_eq!(
        summary(&classification),
        [
            pair(
                ClassificationCode::AdditivityConflict,
                "/semantic_model/0/metrics/1"
            ),
            pair(
                ClassificationCode::AdditivityConflict,
                "/semantic_model/0/metrics/2"
            ),
        ]
    );
    assert!(classification.warnings[0].message.contains("E0G-4.1"));
    assert!(classification.warnings[1].message.contains("E0G-4.2"));
    assert_eq!(
        probe(&classification, "/semantic_model/0/metrics/0"),
        Probe::Classified(Box::new(classification.metrics[0].clone()))
    );
    for expression in ["COUNT(DISTINCT debt.customer_id)", "MEDIAN(debt.balance)"] {
        assert_eq!(
            debt_balance(expression),
            Probe::Unusable(ClassificationCode::AdditivityConflict),
            "{expression}"
        );
    }
    assert_eq!(
        additivity(
            &with_debt_balance_expression("MAX(debt.balance)"),
            DEBT_BALANCE
        )
        .0,
        Additivity::SemiAdditive
    );
}

#[test]
fn e0g_4_2_e0g_01_a_semi_additive_metric_has_one_home_dataset() {
    assert_eq!(
        debt_balance("SUM(debt.balance) / COUNT(customers.customer_id)"),
        Probe::Unusable(ClassificationCode::AdditivityConflict)
    );
    assert_eq!(
        additivity(
            &with_debt_balance_expression("SUM(debt.balance) - MIN(debt.balance)"),
            DEBT_BALANCE
        )
        .0,
        Additivity::SemiAdditive
    );
}

#[test]
fn add_5_1_8_e0g_2_8_an_earlier_rule_comes_first_and_one_warning_per_metric() {
    assert_eq!(
        debt_balance("SUM(debt.balance + customers.customer_id)"),
        Probe::Unusable(ClassificationCode::AmbiguousMeasureGrain)
    );
    assert_eq!(
        debt_balance("1 + 2"),
        Probe::Unusable(ClassificationCode::NoAggregate)
    );
    let classification = classify(&with_debt_balance_expression(
        "AVG(debt.balance) + debt.balance",
    ));
    assert_eq!(
        summary(&classification),
        [pair(
            ClassificationCode::UnaggregatedFinerGrainReference,
            DEBT_BALANCE
        )]
    );
}

#[test]
fn add_5_1_8_a_metric_unusable_after_e0f_is_not_classified() {
    assert_eq!(
        debt_balance("SUM(debt.balance) + 'text'"),
        Probe::NotClassified
    );
}
