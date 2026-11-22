//! SUM-3.2: the components that non-additive and semi-additive metrics of the query read.

use menura_model::expression::SimpleAggregate;
use menura_model::internal_model::{
    Additivity, Component, DeclaredAdditivity, DecomposabilityClass, Descriptions, Metric,
    MetricClassification, NormalizedName,
};
use menura_representations::bound_query::{
    BoundMetric, BoundMetricKind, BoundQuery, BoundQueryParts, MetricPath,
};
use menura_representations::columns::Columns;
use menura_representations::names::OutputName;

use super::eligibility::{checked_with, failed};
use super::expressions::{call, qualified};
use super::{ORDERS, at, by_region_and_month, location, monthly, query, visible_orders};
use crate::rewrites::summaries::condition::SummaryCondition;

/// The metric `name` of the semantic model: with the component `SUM(orders.amount)` when
/// `summed` holds, reading `inputs`, non-additive when `non_additive` holds.
fn metric(name: &str, summed: bool, inputs: &[&str], non_additive: bool) -> Metric {
    let components = if summed {
        vec![Component {
            call: call(SimpleAggregate::Sum, qualified("orders", "amount")),
            filter: None,
            home_dataset: NormalizedName::new(ORDERS),
            class: DecomposabilityClass::Distributive,
        }]
    } else {
        Vec::new()
    };
    Metric {
        name: NormalizedName::new(name),
        expression: None,
        typed: None,
        classification: Some(MetricClassification {
            components,
            inputs: inputs
                .iter()
                .map(|input| NormalizedName::new(input))
                .collect(),
            home_datasets: vec![NormalizedName::new(ORDERS)],
            additivity: Additivity::Additive,
            recomposable: true,
        }),
        declared_additivity: non_additive.then_some(DeclaredAdditivity::NonAdditive),
        derived: None,
        context: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

/// The eligibility of the plan by region and month when the model has `metrics`, ordered by
/// name, for a query that names `named`.
fn with_metrics(metrics: Vec<Metric>, named: Option<&str>) -> Option<SummaryCondition> {
    let mut plan_query = query();
    if let Some(named) = named {
        let metric = BoundMetric {
            kind: BoundMetricKind::Reference(MetricPath::SemanticModel(NormalizedName::new(named))),
            output_name: OutputName::new(named),
            origin: at(),
        };
        plan_query = BoundQuery::new(
            Columns::dimensions_then_metrics(Vec::new(), vec![metric]),
            plan_query.time().clone(),
            BoundQueryParts {
                subgraph: plan_query.subgraph().clone(),
                ..BoundQueryParts::default()
            },
        );
    }
    failed(&checked_with(
        &by_region_and_month(visible_orders()),
        &monthly(),
        |parts| parts.metrics = metrics,
        &plan_query,
    ))
}

fn snapshot() -> Metric {
    metric("AMOUNT_SNAPSHOT", true, &[], true)
}

#[test]
fn sum_3_2_a_component_of_a_non_additive_metric_of_the_query_is_excluded() {
    assert_eq!(
        with_metrics(vec![snapshot()], Some("AMOUNT_SNAPSHOT")),
        Some(SummaryCondition::Exclusions)
    );
}

#[test]
fn sum_3_2_a_non_additive_metric_outside_the_query_excludes_nothing() {
    assert_eq!(with_metrics(vec![snapshot()], None), None);
}

#[test]
fn sum_3_2_a_component_read_first_unrestricted_then_through_a_non_additive_input_is_excluded() {
    // `A_TOTAL` reads `M_AMOUNT` directly, then through the non-additive `R_RATIO`.
    let metrics = vec![
        metric("A_TOTAL", false, &["M_AMOUNT", "R_RATIO"], false),
        metric("M_AMOUNT", true, &[], false),
        metric("R_RATIO", false, &["M_AMOUNT"], true),
    ];
    assert_eq!(
        with_metrics(metrics, Some("A_TOTAL")),
        Some(SummaryCondition::Exclusions)
    );
}
