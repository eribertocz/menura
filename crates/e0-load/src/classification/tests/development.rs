//! The dataset metrics of the development format (section 4.5): the home dataset of each
//! component is the metric's own dataset (E0G-1.4).

use menura_contracts::types::audit_record::OssieFormat;

use super::*;

const DEVELOPMENT_MODEL: &str =
    include_str!("../../../../../conformance/models/e0-metrics-development/model.yaml");
const DEVELOPMENT_CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-metrics-development/catalog-schema.json");

fn classify_development() -> Classification {
    classify_in_format(
        DEVELOPMENT_MODEL,
        DEVELOPMENT_CATALOG,
        OssieFormat::Development,
    )
}

fn dataset_metric(classification: &Classification, index: usize) -> ClassifiedMetric {
    match probe(classification, &format!("/datasets/0/metrics/{index}")) {
        Probe::Classified(metric) => *metric,
        other => panic!("the dataset metric {index} must be classified, found {other:?}"),
    }
}

#[test]
fn e0g_1_4_a_dataset_metric_with_count_rows_is_usable_on_its_own_dataset() {
    let classification = classify_development();
    for index in [0, 1, 2] {
        let metric = dataset_metric(&classification, index);
        assert_eq!(metric.kind, MetricKind::DatasetMetric);
        assert_eq!(
            components(&metric),
            [(DecomposabilityClass::Distributive, orders())],
            "dataset metric {index}"
        );
        assert_eq!(metric.additivity, Additivity::Additive);
    }
}

#[test]
fn e0g_2_dataset_metrics_follow_the_rules_that_apply_to_them() {
    let classification = classify_development();
    assert_eq!(
        summary(&classification),
        [
            pair(
                ClassificationCode::UnaggregatedFinerGrainReference,
                "/datasets/0/metrics/3"
            ),
            pair(ClassificationCode::NoAggregate, "/datasets/0/metrics/4"),
        ]
    );
    assert_eq!(
        probe(&classification, "/datasets/0/metrics/3"),
        Probe::Unusable(ClassificationCode::UnaggregatedFinerGrainReference)
    );
}
