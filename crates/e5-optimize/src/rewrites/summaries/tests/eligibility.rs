//! Each condition of SUM-3 failing alone, and the summaries that pass them all.

use menura_model::expression::{BinaryOperator, CastTarget, DateUnit, Expression, SimpleAggregate};
use menura_model::internal_model::{
    ModelParts, NormalizedName, Summary, SummaryDimension, SummaryMeasure,
};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{PlanNode, Recomposition};
use menura_representations::names::ColumnName;

use super::expressions::{binary, call, cast, column, equals, qualified, truncated};
use super::{
    ORDERS, aggregate, by_region_and_month, component, deduplicate, join, model_with, monthly,
    policy, query, scan, visible_orders, with_context,
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

/// The eligibility of the sum of `argument` over the plan columns, with a summary measure
/// `SUM(base_argument)` over the fields of `ORDERS`.
fn summed(argument: Expression, base_argument: Expression) -> Option<SummaryCondition> {
    let mut summary = monthly();
    summary.measures.push(SummaryMeasure {
        field: NormalizedName::new("AMOUNT_SUM"),
        base_aggregate: Some(Expression::Aggregate {
            call: call(SimpleAggregate::Sum, base_argument),
            filter: None,
        }),
    });
    let mut summed = component("total", SimpleAggregate::Sum, "AMOUNT");
    summed.call = call(SimpleAggregate::Sum, argument);
    let plan = computing(&["AMOUNT"], vec![summed]);
    failed(&checked_with(&plan, &summary, |_| (), &query()))
}

#[test]
fn sum_3_2_an_expression_of_float_type_is_excluded() {
    let argument = cast(column("AMOUNT"), CastTarget::Double);
    let base = cast(qualified("orders", "amount"), CastTarget::Double);
    assert_eq!(summed(argument, base), Some(SummaryCondition::Exclusions));
}

#[test]
fn sum_3_2_an_argument_without_a_known_type_is_excluded() {
    let argument = column("DISCOUNT");
    let base = qualified("orders", "discount");
    assert_eq!(summed(argument, base), Some(SummaryCondition::Exclusions));
}

#[test]
fn sum_3_2_an_exact_expression_is_not_excluded() {
    let argument = cast(column("AMOUNT"), CastTarget::BigInteger);
    let base = cast(qualified("orders", "amount"), CastTarget::BigInteger);
    assert_eq!(summed(argument, base), None);
}

#[test]
fn sum_3_2_a_remaining_deduplicate_is_excluded() {
    let rows = deduplicate(join(visible_orders(), "CUSTOMER_ID"));
    assert_eq!(
        failed(&checked(&by_region_and_month(rows))),
        Some(SummaryCondition::Exclusions)
    );
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

#[test]
fn sum_3_2a_a_zoned_catalog_column_named_as_e0_stores_it_is_found_in_a_lower_case_catalog() {
    let summary = with_dimension("CREATED_DAY", "CREATED_DAY");
    let eligibility = checked_with(
        &by_region_and_month(visible_orders()),
        &summary,
        |parts| {
            let orders = parts
                .datasets
                .iter_mut()
                .find(|dataset| dataset.name.as_str() == ORDERS)
                .expect("the ORDERS dataset");
            orders
                .fields
                .retain(|field| field.name.as_str() != "CREATED_AT");
            let created_day = orders
                .fields
                .iter_mut()
                .find(|field| field.name.as_str() == "CREATED_DAY")
                .expect("the CREATED_DAY field");
            created_day.expression = Some(cast(column("CREATED_AT"), CastTarget::Date));
        },
        &query(),
    );
    assert_eq!(
        failed(&eligibility),
        Some(SummaryCondition::QueryDependentValues)
    );
}
