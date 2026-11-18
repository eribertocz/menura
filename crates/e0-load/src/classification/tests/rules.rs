//! The rules of E0G-2 that E0.g reports, their codes and their precedence (E0G-2.8).

use super::*;

#[test]
fn e0g_2_the_conformance_model_has_one_warning_per_broken_rule() {
    let classification = classify(MODEL);
    assert_eq!(
        summary(&classification),
        [
            pair(
                ClassificationCode::UnaggregatedFinerGrainReference,
                "/semantic_model/0/metrics/8"
            ),
            pair(
                ClassificationCode::UnaggregatedFinerGrainReference,
                "/semantic_model/0/metrics/9"
            ),
            pair(
                ClassificationCode::AmbiguousMeasureGrain,
                "/semantic_model/0/metrics/10"
            ),
            pair(
                ClassificationCode::AmbiguousMeasureGrain,
                "/semantic_model/0/metrics/11"
            ),
            pair(
                ClassificationCode::AmbiguousMeasureGrain,
                "/semantic_model/0/metrics/12"
            ),
            pair(
                ClassificationCode::NoAggregate,
                "/semantic_model/0/metrics/13"
            ),
        ]
    );
    assert_eq!(classification.metrics.len(), 8);
    assert_eq!(classification.unusable.len(), 6);
    for (index, rule) in [(14, "E0G-2.2"), (15, "E0G-2.6")] {
        assert_eq!(
            probe(
                &classification,
                &format!("/semantic_model/0/metrics/{index}")
            ),
            Probe::NotClassified,
            "{rule} is reported by E0.f, not again by E0.g"
        );
    }
    assert_eq!(ClassificationCode::WARNING, "W0G-01");
}

#[test]
fn e0g_2_1_a_reference_outside_every_aggregate_is_an_error() {
    assert_eq!(
        metric("SUM(orders.amount) + orders.amount"),
        Probe::Unusable(ClassificationCode::UnaggregatedFinerGrainReference)
    );
    assert_eq!(
        metric("CASE WHEN orders.status = 'OPEN' THEN SUM(orders.amount) END"),
        Probe::Unusable(ClassificationCode::UnaggregatedFinerGrainReference)
    );
    assert_eq!(
        ClassificationCode::UnaggregatedFinerGrainReference.as_str(),
        "E_UNAGGREGATED_FINER_GRAIN_REFERENCE"
    );
}

#[test]
fn e0g_2_3_arguments_that_read_two_datasets_are_an_error() {
    assert_eq!(
        metric("SUM(orders.amount * customers.age)"),
        Probe::Unusable(ClassificationCode::AmbiguousMeasureGrain)
    );
    assert_eq!(
        ClassificationCode::AmbiguousMeasureGrain.as_str(),
        "E_AMBIGUOUS_MEASURE_GRAIN"
    );
}

#[test]
fn e0g_2_4_a_filter_that_reads_another_dataset_is_an_error() {
    assert_eq!(
        metric("SUM(orders.amount) FILTER (WHERE customers.region = 'north')"),
        Probe::Unusable(ClassificationCode::AmbiguousMeasureGrain)
    );
    assert_eq!(
        metric("SUM(orders.amount) FILTER (WHERE orders.status = 'OPEN' AND customers.age > 1)"),
        Probe::Unusable(ClassificationCode::AmbiguousMeasureGrain)
    );
    assert!(matches!(
        metric("SUM(orders.amount) FILTER (WHERE orders.status = 'OPEN')"),
        Probe::Classified(_)
    ));
    assert!(
        matches!(
            metric("SUM(orders.amount) FILTER (WHERE 1 = 1)"),
            Probe::Classified(_)
        ),
        "a FILTER that reads no dataset reads no other dataset"
    );
}

#[test]
fn e0g_2_5_arguments_that_read_no_dataset_are_an_error_in_a_model_metric() {
    for expression in [
        "COUNT(*)",
        "SUM(1)",
        "COUNT(*) FILTER (WHERE orders.status = 'OPEN')",
        "SUM(orders.amount) / COUNT(*)",
    ] {
        assert_eq!(
            metric(expression),
            Probe::Unusable(ClassificationCode::AmbiguousMeasureGrain),
            "{expression}"
        );
    }
}

#[test]
fn e0g_2_7_a_metric_without_aggregate_or_reference_is_e0g_02() {
    assert_eq!(
        metric("1 + 2"),
        Probe::Unusable(ClassificationCode::NoAggregate)
    );
    assert_eq!(ClassificationCode::NoAggregate.as_str(), "E0G-02");
}

#[test]
fn e0g_2_8_the_first_code_in_the_e2_5_precedence_is_recorded() {
    assert_eq!(
        metric("orders.amount"),
        Probe::Unusable(ClassificationCode::UnaggregatedFinerGrainReference),
        "E0G-2.1 comes before E0G-2.7"
    );
    assert_eq!(
        metric("SUM(orders.amount * customers.age) + orders.amount"),
        Probe::Unusable(ClassificationCode::UnaggregatedFinerGrainReference),
        "E0G-2.1 comes before E0G-2.3"
    );
    assert!(
        ClassificationCode::UnaggregatedFinerGrainReference
            < ClassificationCode::AmbiguousMeasureGrain
    );
    assert!(ClassificationCode::AmbiguousMeasureGrain < ClassificationCode::NoAggregate);
}

#[test]
fn e0g_2_8_one_warning_per_metric_located_at_the_metric() {
    let classification = classify(&document(
        "SUM(orders.amount * customers.age) + COUNT(*) + orders.amount",
    ));
    assert_eq!(
        summary(&classification),
        [pair(
            ClassificationCode::UnaggregatedFinerGrainReference,
            "/semantic_model/0/metrics/0"
        )]
    );
    let location = classification.warnings[0].location;
    assert_eq!(
        (location.line, location.column),
        (12, 9),
        "the metric, not its expression on the next line"
    );
}

#[test]
fn e0g_2_8_metrics_unusable_after_e0f_are_not_classified() {
    for expression in [
        "SUM(orders.amount) OVER (ORDER BY orders.order_id)",
        "SUM(COUNT(orders.amount))",
        "SUM(orders.status)",
        "SUM(orders.missing)",
        "SUM(",
    ] {
        let classification = classify(&document(expression));
        assert_eq!(
            probe(&classification, "/semantic_model/0/metrics/0"),
            Probe::NotClassified,
            "{expression}"
        );
        assert!(classification.warnings.is_empty(), "{expression}");
    }
}
