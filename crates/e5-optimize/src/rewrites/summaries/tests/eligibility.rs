//! Each condition of SUM-3 failing alone, and the summaries that pass them all.

use menura_model::expression::{BinaryOperator, CastTarget, DateUnit, Expression, SimpleAggregate};
use menura_model::internal_model::{
    Additivity, Component, DeclaredAdditivity, DecomposabilityClass, Descriptions, Metric,
    MetricClassification, ModelParts, NormalizedName, Summary, SummaryDimension,
};
use menura_representations::bound_query::{
    BoundMetric, BoundMetricKind, BoundQuery, BoundQueryParts, MetricPath,
};
use menura_representations::columns::Columns;
use menura_representations::logical_plan::{PlanNode, Recomposition};
use menura_representations::names::ColumnName;
use menura_representations::names::OutputName;

use super::expressions::{binary, call, cast, column, equals, qualified, truncated};
use super::{
    ORDERS, aggregate, by_region_and_month, component, deduplicate, join, location, model_with,
    monthly, policy, query, scan, visible_orders, with_context,
};
use crate::rewrites::summaries::condition::SummaryCondition;
use crate::rewrites::summaries::eligibility::{Eligibility, eligibility};

/// The eligibility of `summary` for `branch` in `parts`' model, for `plan_query`.
pub(super) fn checked_with(
    branch: &PlanNode,
    summary: &Summary,
    change: impl FnOnce(&mut ModelParts),
    plan_query: &BoundQuery,
) -> Eligibility {
    let plan_model = model_with(change);
    with_context(&plan_model, plan_query, |context| {
        eligibility(branch, summary, context)
    })
}

/// The eligibility of `ORDERS_MONTHLY` for `branch`, in the fixture model, for `query()`.
pub(super) fn checked(branch: &PlanNode) -> Eligibility {
    checked_with(branch, &monthly(), |_| (), &query())
}

pub(super) fn failed(eligibility: &Eligibility) -> Option<SummaryCondition> {
    match eligibility {
        Eligibility::Eligible(_) => None,
        Eligibility::Ineligible(condition) => Some(*condition),
    }
}

/// The plan by region and month computing `components` over `fields` of the visible orders.
pub(super) fn computing(
    fields: &[&str],
    components: Vec<menura_representations::logical_plan::AggregateComponent>,
) -> PlanNode {
    let mut read = vec!["REGION", "ORDER_DATE", "CUSTOMER_ID"];
    read.extend(fields);
    aggregate(
        policy(
            scan(ORDERS, &read),
            binary(
                BinaryOperator::NotEqual,
                column("REGION"),
                super::expressions::text("XX"),
            ),
        ),
        &[
            ("dimension_1", column("REGION")),
            ("dimension_2", truncated(DateUnit::Month, "ORDER_DATE")),
        ],
        components,
    )
}

/// The plan by region with the key `key`, over the visible orders.
pub(super) fn keyed(key: Expression) -> PlanNode {
    aggregate(
        visible_orders(),
        &[("dimension_1", key)],
        vec![component("total", SimpleAggregate::Sum, "AMOUNT")],
    )
}

#[test]
fn e5_3_5_a_summary_that_meets_every_condition_is_eligible() {
    let eligibility = checked(&by_region_and_month(visible_orders()));
    let Eligibility::Eligible(summary_use) = eligibility else {
        panic!("eligible, got {eligibility:?}");
    };
    assert_eq!(summary_use.summary().as_str(), "ORDERS_MONTHLY");
}

#[test]
fn sum_1_a_summary_of_another_dataset_is_not_eligible() {
    let other = Summary {
        of: NormalizedName::new("CUSTOMERS"),
        ..monthly()
    };
    let eligibility = checked_with(
        &by_region_and_month(visible_orders()),
        &other,
        |_| (),
        &query(),
    );
    assert_eq!(failed(&eligibility), Some(SummaryCondition::Equivalence));
}

#[test]
fn sum_3_1_a_component_without_a_measure_is_not_eligible() {
    let plan = computing(
        &["QUANTITY"],
        vec![component("total", SimpleAggregate::Sum, "QUANTITY")],
    );
    assert_eq!(failed(&checked(&plan)), Some(SummaryCondition::Equivalence));
}

#[test]
fn sum_3_1_holistic_aggregates_never_use_summaries() {
    let plan = computing(
        &["AMOUNT"],
        vec![component("middle", SimpleAggregate::Median, "AMOUNT")],
    );
    assert_eq!(failed(&checked(&plan)), Some(SummaryCondition::Equivalence));
}

#[test]
fn sum_3_1_a_filtered_component_needs_a_measure_with_the_same_filter() {
    let mut filtered = component("total", SimpleAggregate::Sum, "AMOUNT");
    filtered.filter = Some(equals("REGION", "EU"));
    let plan = computing(&["AMOUNT"], vec![filtered]);
    assert_eq!(failed(&checked(&plan)), Some(SummaryCondition::Equivalence));
}

#[test]
fn sum_3_1_count_and_average_recompose_from_sum_and_count() {
    let plan = computing(
        &["AMOUNT"],
        vec![
            component("count", SimpleAggregate::Count, "AMOUNT"),
            component("average", SimpleAggregate::Average, "AMOUNT"),
            component("largest", SimpleAggregate::Maximum, "AMOUNT"),
        ],
    );
    let eligibility = checked(&plan);
    let Eligibility::Eligible(summary_use) = eligibility else {
        panic!("eligible, got {eligibility:?}");
    };
    let recomposed: Vec<_> = summary_use
        .components
        .iter()
        .map(|component| {
            (
                component.call.clone(),
                component.recomposition.clone(),
                component.filter.clone(),
            )
        })
        .collect();
    assert_eq!(
        recomposed,
        vec![
            (
                call(SimpleAggregate::Sum, column("AMOUNT_COUNT")),
                Some(Recomposition::Count),
                None
            ),
            (
                call(SimpleAggregate::Sum, column("AMOUNT_SUM")),
                Some(Recomposition::Average {
                    count: ColumnName::new("AMOUNT_COUNT"),
                }),
                None,
            ),
            (
                call(SimpleAggregate::Maximum, column("AMOUNT_MAX")),
                Some(Recomposition::MinMax),
                None
            ),
        ]
    );
}

#[test]
fn sum_3_2_a_float_argument_is_excluded() {
    let plan = computing(
        &["PRICE"],
        vec![component("total", SimpleAggregate::Sum, "PRICE")],
    );
    assert_eq!(failed(&checked(&plan)), Some(SummaryCondition::Exclusions));
}

#[test]
fn sum_3_2_a_remaining_deduplicate_is_excluded() {
    let rows = deduplicate(join(visible_orders(), "CUSTOMER_ID"));
    assert_eq!(
        failed(&checked(&by_region_and_month(rows))),
        Some(SummaryCondition::Exclusions)
    );
}

/// The non-additive metric `AMOUNT_SNAPSHOT`, whose component is `SUM(orders.amount)`.
fn non_additive() -> Metric {
    Metric {
        name: NormalizedName::new("AMOUNT_SNAPSHOT"),
        expression: None,
        typed: None,
        classification: Some(MetricClassification {
            components: vec![Component {
                call: call(SimpleAggregate::Sum, qualified("orders", "amount")),
                filter: None,
                home_dataset: NormalizedName::new(ORDERS),
                class: DecomposabilityClass::Distributive,
            }],
            inputs: Vec::new(),
            home_datasets: vec![NormalizedName::new(ORDERS)],
            additivity: Additivity::Additive,
            recomposable: true,
        }),
        declared_additivity: Some(DeclaredAdditivity::NonAdditive),
        derived: None,
        context: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

/// The eligibility of the plan by region and month when the model has `non_additive`, for a
/// query that names it when `named` holds.
fn with_non_additive(named: bool) -> Option<SummaryCondition> {
    let mut plan_query = query();
    if named {
        let metric = BoundMetric {
            kind: BoundMetricKind::Reference(MetricPath::SemanticModel(NormalizedName::new(
                "AMOUNT_SNAPSHOT",
            ))),
            output_name: OutputName::new("AMOUNT_SNAPSHOT"),
            origin: super::at(),
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
        |parts| parts.metrics.push(non_additive()),
        &plan_query,
    ))
}

#[test]
fn sum_3_2_a_component_of_a_non_additive_metric_of_the_query_is_excluded() {
    assert_eq!(with_non_additive(true), Some(SummaryCondition::Exclusions));
}

#[test]
fn sum_3_2_a_non_additive_metric_outside_the_query_excludes_nothing() {
    assert_eq!(with_non_additive(false), None);
}

#[test]
fn sum_3_2_a_semi_additive_branch_is_excluded() {
    let not_an_aggregate = visible_orders();
    assert_eq!(
        failed(&checked(&not_an_aggregate)),
        Some(SummaryCondition::Exclusions)
    );
}

#[test]
fn sum_3_2a_a_dimension_that_uses_the_current_date_is_not_eligible() {
    let eligibility = checked_with(
        &by_region_and_month(visible_orders()),
        &monthly(),
        |parts| {
            let orders = parts
                .datasets
                .iter_mut()
                .find(|dataset| dataset.name.as_str() == ORDERS);
            let region = orders
                .and_then(|orders| {
                    orders
                        .fields
                        .iter_mut()
                        .find(|field| field.name.as_str() == "REGION")
                })
                .expect("the REGION field");
            region.expression = Some(cast(Expression::CurrentDate, CastTarget::Text));
        },
        &query(),
    );
    assert_eq!(
        failed(&eligibility),
        Some(SummaryCondition::QueryDependentValues)
    );
}

fn with_dimension(field: &str, base: &str) -> Summary {
    let mut summary = monthly();
    summary.dimensions.push(SummaryDimension {
        field: NormalizedName::new(field),
        base: NormalizedName::new(base),
        grain: Some(menura_contracts::types::common::Grain::Day),
    });
    summary
}

#[test]
fn sum_3_2a_a_dimension_computed_from_a_zoned_timestamp_is_not_eligible() {
    let summary = with_dimension("CREATED_DAY", "CREATED_DAY");
    let eligibility = checked_with(
        &by_region_and_month(visible_orders()),
        &summary,
        |_| (),
        &query(),
    );
    assert_eq!(
        failed(&eligibility),
        Some(SummaryCondition::QueryDependentValues)
    );
}

#[test]
fn sum_3_2a_a_zoned_timestamp_as_the_plain_column_of_a_time_dimension_is_eligible() {
    let summary = with_dimension("CREATED_AT", "CREATED_AT");
    let eligibility = checked_with(
        &by_region_and_month(visible_orders()),
        &summary,
        |_| (),
        &query(),
    );
    assert_eq!(failed(&eligibility), None);
}
