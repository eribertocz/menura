//! Unit tests of derived metrics in path resolution (DER-2, DER-4, E0G-5.2), named after the
//! rule they check.

use menura_model::expression::{BinaryOperator, Expression, Identifier, Reference};
use menura_model::internal_model::{Dataset, Metric, MetricClassification};

use super::*;
use crate::{PathCode, PathDiagnostic};

/// The normalized identifier `text`, as E0 writes it in a model tree (E0H-1.3).
fn identifier(text: &str) -> Identifier {
    Identifier::delimited(text).expect("a non-empty identifier")
}

/// `AGG(metric)`, or `AGG(dataset.metric)` with a `dataset`.
fn agg(dataset: Option<&str>, metric: &str) -> Expression {
    Expression::MetricAggregate(Reference {
        qualifier: dataset.map(identifier),
        name: identifier(metric),
    })
}

fn divided(left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator: BinaryOperator::Divide,
        left: Box::new(left),
        right: Box::new(right),
    }
}

/// The derived metric `metric_name` with the formula `formula` over the home datasets `homes`,
/// classified as E0.g does: no components of its own (E0G-5.2, DER-5.1.6).
fn derived(metric_name: &str, formula: Expression, homes: &[&str]) -> Metric {
    let mut metric = metric(metric_name, &[]);
    metric.expression = Some(formula);
    metric.classification = Some(MetricClassification {
        home_datasets: homes.iter().map(|home| name(home)).collect(),
        ..metric.classification.expect("a classification")
    });
    metric
}

/// The model with `datasets` and the semantic-model `metrics`, ordered by name as the internal
/// model orders them (E0H-2.3).
fn sorted_model(datasets: Vec<Dataset>, mut metrics: Vec<Metric>) -> InternalModel {
    metrics.sort_by(|left, right| left.name.cmp(&right.name));
    model(datasets, metrics)
}

fn branch_components(branches: &Branches) -> Vec<(&str, &[ComponentKey])> {
    branches
        .as_slice()
        .iter()
        .map(|branch| (branch.home.as_str(), branch.components.as_slice()))
        .collect()
}

fn only_metric(metric_name: &str) -> menura_representations::bound_query::BoundQuery {
    query(
        Vec::new(),
        vec![metric_reference(metric_name, 0)],
        BoundQueryParts::default(),
    )
}

#[test]
fn der_2_1_a_derived_metric_is_expanded_into_the_components_of_its_inputs() {
    let model = sorted_model(
        Vec::new(),
        vec![
            metric("BILLED", &["INVOICES"]),
            derived(
                "COLLECTION_RATE",
                divided(agg(None, "COLLECTED"), agg(None, "BILLED")),
                &["INVOICES"],
            ),
            metric("COLLECTED", &["INVOICES"]),
        ],
    );

    let branches = resolve_paths(&only_metric("COLLECTION_RATE"), &model).expect("one branch");

    assert_eq!(
        branch_components(&branches),
        [("INVOICES", &[key(0, 0), key(0, 1)][..])]
    );
}

#[test]
fn der_4_3_inputs_from_different_home_datasets_are_separate_branches() {
    let model = sorted_model(
        Vec::new(),
        vec![
            metric("BUDGET", &["BUDGETS"]),
            derived(
                "SALES_TO_BUDGET",
                divided(agg(None, "SALES"), agg(None, "BUDGET")),
                &["BUDGETS", "ORDERS"],
            ),
            metric("SALES", &["ORDERS"]),
        ],
    );

    let branches = resolve_paths(&only_metric("SALES_TO_BUDGET"), &model).expect("two branches");

    assert_eq!(
        branch_components(&branches),
        [("BUDGETS", &[key(0, 1)][..]), ("ORDERS", &[key(0, 0)][..])]
    );
}

#[test]
fn der_2_2_agg_of_a_dataset_metric_reads_the_components_of_that_dataset_metric() {
    let model = sorted_model(
        vec![
            dataset("ORDERS", vec![metric("REVENUE", &["ORDERS"])]),
            dataset("RETURNS", vec![metric("REVENUE", &["RETURNS"])]),
        ],
        vec![derived(
            "RETURN_RATE",
            divided(
                agg(Some("RETURNS"), "REVENUE"),
                agg(Some("ORDERS"), "REVENUE"),
            ),
            &["ORDERS", "RETURNS"],
        )],
    );

    let branches = resolve_paths(&only_metric("RETURN_RATE"), &model).expect("two branches");

    assert_eq!(
        branch_components(&branches),
        [("ORDERS", &[key(0, 1)][..]), ("RETURNS", &[key(0, 0)][..])]
    );
}

#[test]
fn der_2_4_a_derived_metric_over_a_derived_metric_is_expanded_down_to_base_components() {
    let model = sorted_model(
        Vec::new(),
        vec![
            metric("BILLED", &["INVOICES"]),
            derived(
                "COLLECTION_RATE",
                divided(agg(None, "COLLECTED"), agg(None, "BILLED")),
                &["INVOICES"],
            ),
            metric("COLLECTED", &["INVOICES"]),
            derived(
                "RATE_PER_REFUND",
                divided(agg(None, "COLLECTION_RATE"), agg(None, "REFUNDS")),
                &["INVOICES", "REFUNDS"],
            ),
            metric("REFUNDS", &["REFUNDS"]),
        ],
    );

    let branches = resolve_paths(&only_metric("RATE_PER_REFUND"), &model).expect("two branches");

    assert_eq!(
        branch_components(&branches),
        [
            ("INVOICES", &[key(0, 0), key(0, 1)][..]),
            ("REFUNDS", &[key(0, 2)][..])
        ]
    );
}

#[test]
fn der_2_4_an_input_read_twice_is_expanded_once() {
    let model = sorted_model(
        Vec::new(),
        vec![
            metric("BILLED", &["INVOICES"]),
            derived(
                "SQUARED_SHARE",
                divided(
                    agg(None, "BILLED"),
                    Expression::Binary {
                        operator: BinaryOperator::Multiply,
                        left: Box::new(agg(None, "BILLED")),
                        right: Box::new(agg(None, "BILLED")),
                    },
                ),
                &["INVOICES"],
            ),
        ],
    );

    let branches = resolve_paths(&only_metric("SQUARED_SHARE"), &model).expect("one branch");

    assert_eq!(
        branch_components(&branches),
        [("INVOICES", &[key(0, 0)][..])]
    );
}

#[test]
fn e0g_5_2_a_dimension_must_be_reachable_from_every_home_dataset_of_a_derived_metric() {
    let model = connected_model(
        Vec::new(),
        vec![
            metric("BUDGET", &["BUDGETS"]),
            metric("SALES", &["ORDERS"]),
            derived(
                "SALES_TO_BUDGET",
                divided(agg(None, "SALES"), agg(None, "BUDGET")),
                &["BUDGETS", "ORDERS"],
            ),
        ],
        &[("ORDERS_CUSTOMERS", "ORDERS", "CUSTOMERS")],
    );
    let query = query(
        vec![dimension("CUSTOMERS", "COUNTRY", 0)],
        vec![metric_reference("SALES_TO_BUDGET", 0)],
        authorized(BoundQueryParts::default()),
    );

    let error = resolve_paths(&query, &model).expect_err("no metric reaches CUSTOMERS");

    let PathError::Diagnostic(PathDiagnostic { code, .. }) = error else {
        panic!("a diagnostic, got {error:?}");
    };
    assert_eq!(code, PathCode::NoPath);
}

#[test]
fn der_2_4_a_derived_metric_that_names_a_missing_metric_is_an_invariant_violation() {
    let model = sorted_model(
        Vec::new(),
        vec![derived(
            "BROKEN",
            divided(agg(None, "MISSING"), agg(None, "MISSING")),
            &["INVOICES"],
        )],
    );

    assert_eq!(
        resolve_paths(&only_metric("BROKEN"), &model),
        violation(InvariantViolation::UnclassifiedMetric {
            origin: at("/query/metrics/0"),
        })
    );
}

#[test]
fn der_2_4_a_cycle_of_derived_metrics_is_an_invariant_violation() {
    let model = sorted_model(
        Vec::new(),
        vec![derived(
            "LOOP",
            divided(agg(None, "LOOP"), agg(None, "LOOP")),
            &["INVOICES"],
        )],
    );

    assert_eq!(
        resolve_paths(&only_metric("LOOP"), &model),
        violation(InvariantViolation::DerivedCycle {
            origin: at("/query/metrics/0"),
        })
    );
}
