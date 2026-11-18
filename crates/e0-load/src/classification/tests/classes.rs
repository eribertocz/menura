//! Components and home datasets (E0G-1.1), decomposability (E0G-3), the deduced additivity
//! (E0G-4) and the output (E0G-6).

use semantic_compiler_contracts::types::common::{FloatBits, IntegerBits, LogicalType};

use super::*;

#[test]
fn e0g_1_1_each_aggregate_is_a_component_with_its_home_dataset() {
    let metric = classified("SUM(orders.amount) / COUNT(DISTINCT customers.customer_id)");
    assert_eq!(
        components(&metric),
        [
            (DecomposabilityClass::Distributive, orders()),
            (DecomposabilityClass::Holistic, customers()),
        ]
    );
    assert_eq!(metric.kind, MetricKind::Metric);
}

#[test]
fn e0g_3_classes_follow_the_decomposability_reference() {
    let class_of = |expression: &str| {
        let metric = classified(expression);
        assert_eq!(metric.components.len(), 1, "{expression}");
        metric.components[0].class
    };
    for expression in [
        "SUM(orders.amount)",
        "COUNT(orders.order_id)",
        "MIN(orders.amount)",
        "MAX(orders.amount)",
    ] {
        assert_eq!(
            class_of(expression),
            DecomposabilityClass::Distributive,
            "{expression}"
        );
    }
    for expression in [
        "AVG(customers.age)",
        "STDDEV(customers.age)",
        "STDDEV_POP(customers.age)",
        "VAR_SAMP(customers.age)",
        "VARIANCE(customers.age)",
    ] {
        assert_eq!(
            class_of(expression),
            DecomposabilityClass::Algebraic,
            "{expression}"
        );
    }
    for expression in [
        "MEDIAN(customers.age)",
        "PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY customers.age)",
        "COUNT(DISTINCT orders.customer_id)",
        "SUM(DISTINCT orders.amount)",
    ] {
        assert_eq!(
            class_of(expression),
            DecomposabilityClass::Holistic,
            "{expression}"
        );
    }
}

#[test]
fn e0g_3_1_a_metric_is_recomposable_only_if_every_component_is() {
    assert!(classified("SUM(orders.amount)").recomposable);
    assert!(classified("MAX(orders.amount) - MIN(orders.amount)").recomposable);
    assert!(
        classified("AVG(customers.age)").recomposable,
        "AVG is SUM / COUNT"
    );
    assert!(!classified("STDDEV(customers.age)").recomposable);
    assert!(!classified("MEDIAN(customers.age)").recomposable);
    assert!(!classified("SUM(orders.amount) / COUNT(DISTINCT customers.customer_id)").recomposable);
}

#[test]
fn e0g_3_2_a_filter_does_not_change_the_class() {
    let metric =
        classified("COUNT(DISTINCT orders.customer_id) FILTER (WHERE orders.status = 'OPEN')");
    assert_eq!(metric.components[0].class, DecomposabilityClass::Holistic);
    assert!(metric.components[0].filter.is_some());
    let metric = classified("SUM(orders.amount) FILTER (WHERE orders.status = 'OPEN')");
    assert_eq!(
        metric.components[0].class,
        DecomposabilityClass::Distributive
    );
    assert_eq!(metric.additivity, Additivity::Additive);
}

#[test]
fn e0g_4_additivity_is_deduced_from_the_classes() {
    assert_eq!(
        classified("SUM(orders.amount)").additivity,
        Additivity::Additive
    );
    assert_eq!(
        classified("SUM(orders.amount) + SUM(orders.order_id)").additivity,
        Additivity::Additive
    );
    assert_eq!(
        classified("AVG(customers.age)").additivity,
        Additivity::NonAdditive
    );
    assert_eq!(
        classified("COUNT(DISTINCT orders.customer_id)").additivity,
        Additivity::NonAdditive
    );
    assert_eq!(
        classified("SUM(orders.amount) / COUNT(DISTINCT customers.customer_id)").additivity,
        Additivity::NonAdditive
    );
}

#[test]
fn e0g_6_the_result_type_and_nullability_come_from_e0f() {
    let metric = classified("AVG(customers.age)");
    assert_eq!(
        metric.result_type,
        LogicalType::Float {
            bits: FloatBits::SixtyFour
        }
    );
    assert!(metric.nullable);
    let metric = classified("COUNT(orders.order_id)");
    assert_eq!(
        metric.result_type,
        LogicalType::Integer {
            bits: IntegerBits::SixtyFour,
            signed: true
        }
    );
    assert!(!metric.nullable);
}

#[test]
fn e0g_6_the_conformance_model_metrics_are_classified_in_document_order() {
    let classification = classify(MODEL);
    let pointers: Vec<String> = classification
        .metrics
        .iter()
        .map(|metric| metric.pointer.to_string())
        .collect();
    let expected: Vec<String> = (0..8)
        .map(|index| format!("/semantic_model/0/metrics/{index}"))
        .collect();
    assert_eq!(pointers, expected);
    assert_eq!(
        components(&classification.metrics[1]),
        [(DecomposabilityClass::Algebraic, customers())],
        "AVG(customers.age)"
    );
    assert_eq!(
        components(&classification.metrics[2]),
        [(DecomposabilityClass::Holistic, orders())],
        "COUNT(DISTINCT orders.customer_id)"
    );
}
