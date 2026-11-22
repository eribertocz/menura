//! The branch that reads a summary (E5-3.5a): its row-policy nodes (E5-3.7), its mapped
//! references and its recomposed components (SUM-3.1, SUM-3.8).

use menura_model::expression::{BinaryOperator, DateUnit, SimpleAggregate};
use menura_representations::logical_plan::{AggregateComponent, PlanNode, Recomposition};

use super::expressions::{binary, call, column, equals, text, truncated};
use super::{
    MONTHLY, ORDERS, aggregate, and, component, date, date_range_term, join, model, policy,
    query_filter, query_of, scan, semi_join, with_context,
};
use crate::rewrites::summaries::eligibility::{Eligibility, eligibility};
use crate::rewrites::summaries::summary_branch::summary_branch;

/// The rows of `home`, read by `scan`, under the row policy on `region` and the propagated
/// policy from `customer`, joined to `CUSTOMERS` and filtered by region and the first quarter
/// of 2026 on `date_column`.
fn rows(home: PlanNode, region: &str, customer: &str, date_column: &str) -> PlanNode {
    let visible = semi_join(
        policy(
            home,
            binary(BinaryOperator::NotEqual, column(region), text("XX")),
        ),
        customer,
    );
    let predicate = and(
        equals(region, "EU"),
        date_range_term(date_column, date(2026, 1, 1), date(2026, 3, 31)),
    );
    query_filter(join(visible, customer), predicate)
}

/// The `Aggregate` by country and quarter of `components` over `input`.
fn by_country_and_quarter(
    input: PlanNode,
    date_column: &str,
    components: Vec<AggregateComponent>,
) -> PlanNode {
    aggregate(
        input,
        &[
            ("dimension_1", column("j1.COUNTRY")),
            ("dimension_2", truncated(DateUnit::Quarter, date_column)),
        ],
        components,
    )
}

fn base_branch() -> PlanNode {
    let home = scan(ORDERS, &["REGION", "ORDER_DATE", "CUSTOMER_ID", "AMOUNT"]);
    by_country_and_quarter(
        rows(home, "REGION", "CUSTOMER_ID", "ORDER_DATE"),
        "ORDER_DATE",
        vec![component("total", SimpleAggregate::Sum, "AMOUNT")],
    )
}

/// The summary branch of `branch` over `ORDERS_MONTHLY`, for the query of `base_branch`.
fn rewritten(branch: &PlanNode) -> PlanNode {
    let plan_model = model();
    let plan_query = query_of(
        &[equals("REGION", "EU")],
        Some(("ORDER_DATE", date(2026, 1, 1), date(2026, 3, 31))),
        true,
    );
    let summary = super::monthly();
    let eligibility = with_context(&plan_model, &plan_query, |context| {
        eligibility(branch, &summary, context)
    });
    let Eligibility::Eligible(summary_use) = eligibility else {
        panic!("eligible, got {eligibility:?}");
    };
    summary_branch(branch, &summary_use).expect("a summary branch")
}

#[test]
fn e5_3_5a_the_summary_branch_reads_the_summary_with_mapped_fields() {
    let home = scan(MONTHLY, &["AMOUNT_SUM", "CUSTOMER_ID", "MONTH", "REGION"]);
    let recomposed = AggregateComponent {
        name: menura_representations::names::ColumnName::new("total"),
        call: call(SimpleAggregate::Sum, column("AMOUNT_SUM")),
        filter: None,
        recomposition: Some(Recomposition::Sum),
    };
    let expected = by_country_and_quarter(
        rows(home, "REGION", "CUSTOMER_ID", "MONTH"),
        "MONTH",
        vec![recomposed],
    );
    assert_eq!(rewritten(&base_branch()), expected);
}

/// The row-policy nodes of `plan`: its `Filter` nodes of model origin and its `SemiJoin`
/// nodes, with their inputs left out.
fn policy_nodes(plan: &PlanNode) -> Vec<String> {
    use menura_representations::logical_plan::Operation;
    use menura_representations::origin::Origin;
    let mut found = Vec::new();
    let mut pending = vec![plan];
    while let Some(node) = pending.pop() {
        match (node.operation(), node.origin()) {
            (Operation::Filter { predicate, .. }, Origin::Model(_)) => {
                found.push(format!("Filter {predicate:?}"));
            }
            (Operation::SemiJoin { relationship, .. }, _) => {
                found.push(format!("SemiJoin {}", relationship.as_str()));
            }
            _ => {}
        }
        pending.extend(node.inputs());
    }
    found
}

#[test]
fn e5_3_7_the_summary_branch_keeps_every_row_policy_node_of_the_base() {
    let base = policy_nodes(&base_branch());
    let summary = policy_nodes(&rewritten(&base_branch()));
    assert_eq!(base.len(), summary.len());
    assert_eq!(
        base.len(),
        3,
        "the own filter, the semi-join and its visible rows"
    );
    assert!(summary.contains(&"SemiJoin ORDERS_TO_CUSTOMERS".to_owned()));
}

#[test]
fn sum_3_8_the_summary_branch_keeps_the_output_schema_of_the_base() {
    let base = base_branch();
    assert_eq!(rewritten(&base).schema(), base.schema());
}

#[test]
fn sum_4_1_the_summary_branch_outputs_no_summary_field() {
    let summary_fields = [
        "MONTH",
        "AMOUNT_SUM",
        "AMOUNT_COUNT",
        "AMOUNT_MAX",
        "PRICE_SUM",
    ];
    let rewritten = rewritten(&base_branch());
    assert!(
        rewritten
            .schema()
            .columns()
            .iter()
            .all(|column| !summary_fields.contains(&column.name.as_str()))
    );
}
