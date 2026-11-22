//! SUM-3.3 to SUM-3.6 and SUM-3.8 failing alone, and the branches that pass them.

use menura_model::expression::{BinaryOperator, DateUnit, SimpleAggregate};

use super::eligibility::{checked, checked_with, computing, failed, keyed};
use super::expressions::{binary, column, equals, is_set, truncated};
use super::{
    ORDERS, aggregate, and, by_region_and_month, component, date, date_range_term, join, monthly,
    policy, query, query_filter, query_of, scan, semi_join, visible_orders,
};
use crate::rewrites::summaries::condition::SummaryCondition;

#[test]
fn sum_3_3_a_dimension_the_summary_does_not_keep_is_not_eligible() {
    let plan = aggregate(
        policy(
            scan(ORDERS, &["REGION", "AMOUNT", "QUANTITY"]),
            binary(
                BinaryOperator::NotEqual,
                column("REGION"),
                super::expressions::text("XX"),
            ),
        ),
        &[("dimension_1", column("QUANTITY"))],
        vec![component("total", SimpleAggregate::Sum, "AMOUNT")],
    );
    assert_eq!(failed(&checked(&plan)), Some(SummaryCondition::Grain));
}

#[test]
fn sum_3_3_a_join_from_columns_the_summary_does_not_keep_is_not_eligible() {
    let rows = join(
        scan(ORDERS, &["REGION", "ORDER_DATE", "SALES_REP_ID", "AMOUNT"]),
        "SALES_REP_ID",
    );
    let plan = aggregate(
        rows,
        &[("dimension_1", column("j1.COUNTRY"))],
        vec![component("total", SimpleAggregate::Sum, "AMOUNT")],
    );
    assert_eq!(failed(&checked(&plan)), Some(SummaryCondition::Grain));
}

#[test]
fn sum_3_3_a_dimension_reached_through_kept_from_columns_is_eligible() {
    let plan = aggregate(
        join(visible_orders(), "CUSTOMER_ID"),
        &[("dimension_1", column("j1.COUNTRY"))],
        vec![component("total", SimpleAggregate::Sum, "AMOUNT")],
    );
    assert_eq!(failed(&checked(&plan)), None);
}

#[test]
fn sum_3_3_a_date_range_on_a_field_the_summary_does_not_keep_is_not_eligible() {
    let (from, to) = (date(2026, 1, 1), date(2026, 3, 31));
    let rows = query_filter(
        policy(
            scan(ORDERS, &["REGION", "ORDER_DATE", "AMOUNT", "SHIPPED_DATE"]),
            equals("REGION", "EU"),
        ),
        date_range_term("SHIPPED_DATE", from, to),
    );
    let plan_query = query_of(&[], Some(("SHIPPED_DATE", from, to)), true);
    let eligibility = checked_with(&by_region_and_month(rows), &monthly(), |_| (), &plan_query);
    assert_eq!(failed(&eligibility), Some(SummaryCondition::Grain));
}

#[test]
fn sum_3_3a_a_filter_on_a_field_with_grain_is_not_eligible() {
    let rows = query_filter(visible_orders(), is_set("ORDER_DATE"));
    let plan_query = query_of(&[is_set("ORDER_DATE")], None, true);
    let eligibility = checked_with(&by_region_and_month(rows), &monthly(), |_| (), &plan_query);
    assert_eq!(
        failed(&eligibility),
        Some(SummaryCondition::TruncatedFields)
    );
}

#[test]
fn sum_3_3a_a_dimension_without_time_grain_on_a_field_with_grain_is_not_eligible() {
    assert_eq!(
        failed(&checked(&keyed(column("ORDER_DATE")))),
        Some(SummaryCondition::TruncatedFields)
    );
}

#[test]
fn sum_3_3a_a_row_policy_on_a_field_with_grain_is_not_eligible() {
    let rows = policy(
        scan(ORDERS, &["REGION", "ORDER_DATE", "AMOUNT"]),
        is_set("ORDER_DATE"),
    );
    assert_eq!(
        failed(&checked(&by_region_and_month(rows))),
        Some(SummaryCondition::TruncatedFields)
    );
}

#[test]
fn sum_3_4_a_week_query_over_a_month_summary_is_not_eligible() {
    assert_eq!(
        failed(&checked(&keyed(truncated(DateUnit::Week, "ORDER_DATE")))),
        Some(SummaryCondition::TimeGrain)
    );
}

#[test]
fn sum_3_4_a_quarter_query_over_a_month_summary_is_eligible() {
    assert_eq!(
        failed(&checked(&keyed(truncated(DateUnit::Quarter, "ORDER_DATE")))),
        None
    );
}

/// The eligibility for the plan by region and month with a date range on `ORDER_DATE` and
/// the query filter `REGION = 'EU'`.
fn in_range(
    from: menura_model::expression::DateValue,
    to: menura_model::expression::DateValue,
) -> Option<SummaryCondition> {
    let predicate = and(
        equals("REGION", "EU"),
        date_range_term("ORDER_DATE", from, to),
    );
    let rows = query_filter(visible_orders(), predicate);
    let plan_query = query_of(
        &[equals("REGION", "EU")],
        Some(("ORDER_DATE", from, to)),
        true,
    );
    failed(&checked_with(
        &by_region_and_month(rows),
        &monthly(),
        |_| (),
        &plan_query,
    ))
}

#[test]
fn sum_3_5_a_date_range_not_aligned_on_the_grain_is_not_eligible() {
    assert_eq!(
        in_range(date(2026, 1, 15), date(2026, 3, 31)),
        Some(SummaryCondition::TimeFilterAlignment)
    );
    assert_eq!(
        in_range(date(2026, 1, 1), date(2026, 3, 30)),
        Some(SummaryCondition::TimeFilterAlignment)
    );
}

#[test]
fn sum_3_5_a_date_range_aligned_on_the_grain_is_eligible() {
    assert_eq!(in_range(date(2026, 1, 1), date(2026, 3, 31)), None);
    assert_eq!(in_range(date(2026, 1, 1), date(9999, 12, 31)), None);
}

#[test]
fn sum_3_6_a_policy_on_a_field_the_summary_does_not_keep_is_not_eligible() {
    let rows = policy(
        scan(ORDERS, &["REGION", "ORDER_DATE", "SALES_REP_ID", "AMOUNT"]),
        equals("SALES_REP_ID", "7"),
    );
    assert_eq!(
        failed(&checked(&by_region_and_month(rows))),
        Some(SummaryCondition::PolicyPreservation)
    );
}

#[test]
fn sum_3_6_a_propagated_policy_whose_columns_are_not_kept_is_not_eligible() {
    let rows = semi_join(
        scan(ORDERS, &["REGION", "ORDER_DATE", "SALES_REP_ID", "AMOUNT"]),
        "SALES_REP_ID",
    );
    assert_eq!(
        failed(&checked(&by_region_and_month(rows))),
        Some(SummaryCondition::PolicyPreservation)
    );
}

#[test]
fn sum_3_6_a_propagated_policy_whose_columns_are_kept_is_eligible() {
    let rows = semi_join(visible_orders(), "CUSTOMER_ID");
    assert_eq!(failed(&checked(&by_region_and_month(rows))), None);
}

#[test]
fn sum_3_6_a_user_without_access_to_the_base_is_not_eligible() {
    let eligibility = checked_with(
        &by_region_and_month(visible_orders()),
        &monthly(),
        |_| (),
        &query_of(&[], None, false),
    );
    assert_eq!(
        failed(&eligibility),
        Some(SummaryCondition::PolicyPreservation)
    );
}

#[test]
fn sum_3_8_a_summary_field_without_a_type_is_not_eligible() {
    let eligibility = checked_with(
        &by_region_and_month(visible_orders()),
        &monthly(),
        |parts| {
            let summary = parts
                .datasets
                .iter_mut()
                .find(|dataset| dataset.name.as_str() == "ORDERS_MONTHLY")
                .and_then(|dataset| {
                    dataset
                        .fields
                        .iter_mut()
                        .find(|field| field.name.as_str() == "AMOUNT_SUM")
                })
                .expect("the AMOUNT_SUM field");
            summary.typed = None;
        },
        &query(),
    );
    assert_eq!(failed(&eligibility), Some(SummaryCondition::SameType));
}

#[test]
fn sum_3_conditions_are_checked_in_order() {
    let plan = computing(
        &["PRICE"],
        vec![component("total", SimpleAggregate::Median, "PRICE")],
    );
    assert_eq!(failed(&checked(&plan)), Some(SummaryCondition::Equivalence));
    assert_eq!(SummaryCondition::QueryDependentValues.rule_id(), "SUM-3.2a");
}
