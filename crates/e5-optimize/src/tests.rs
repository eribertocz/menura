//! Unit tests of the optimize stage (E5-1, E5-2, E5-3, E5-3.7, E5-3.8).

use menura_contracts::types::catalog::CatalogStateDocument;
use serde_json::json;

use crate::rewrites::PIPELINE;
use crate::test_support::{anyone, empty_query, filtered_scan, shop_model, shop_schema};
use crate::{RewriteRule, optimize};

/// A state with two relations, listed in reverse order when `reversed`; the members of the
/// two relation objects are written in different orders.
fn state_in_order(reversed: bool) -> CatalogStateDocument {
    let orders = json!({"source": "shop.main.orders", "row_count_estimate": 10,
        "version": {"origin": "warehouse:shop", "snapshot": "orders-1"}});
    let customers = json!({"version": {"snapshot": "customers-1", "origin": "warehouse:shop"},
        "row_count_estimate": 2, "source": "shop.main.customers"});
    let relations = if reversed {
        json!([customers, orders])
    } else {
        json!([orders, customers])
    };
    serde_json::from_value(json!({
        "contract": "menura.catalog_state",
        "contract_version": "1.1.0",
        "document_id": {"origin": "warehouse:shop", "snapshot": "state-1"},
        "schema_document_id": {"origin": "warehouse:shop", "snapshot": "schema-1"},
        "relations": relations
    }))
    .expect("a well-formed catalog state")
}

#[test]
fn e5_3_the_rewrites_run_in_the_order_of_e5_3() {
    let rules: Vec<&str> = PIPELINE.iter().map(|(rule, _)| rule.rule_id()).collect();

    assert_eq!(
        rules,
        ["E5-3.1", "E5-3.2", "E5-3.3", "E5-3.4", "E5-3.5", "E5-3.6"]
    );
}

#[test]
fn e5_3_8_each_rewrite_rule_names_its_rule_id() {
    let pairs = [
        (RewriteRule::RemoveUnusedJoins, "E5-3.1"),
        (RewriteRule::SkipDeduplication, "E5-3.2"),
        (RewriteRule::InnerJoins, "E5-3.3"),
        (RewriteRule::MergeScans, "E5-3.4"),
        (RewriteRule::UseSummary, "E5-3.5"),
        (RewriteRule::FoldConstants, "E5-3.6"),
    ];

    for (rule, rule_id) in pairs {
        assert_eq!(rule.rule_id(), rule_id);
    }
}

#[test]
fn e5_1_e5_2_2_without_an_applicable_rewrite_the_plan_is_unchanged() {
    let plan = filtered_scan();

    let optimized = optimize(
        &plan,
        &empty_query(),
        &shop_model(),
        &shop_schema(),
        None,
        &anyone(),
    );

    assert_eq!(optimized.plan(), &plan);
    assert!(optimized.records().is_empty());
}

#[test]
fn e5_2_1_a_state_is_optional() {
    let plan = filtered_scan();
    let state = state_in_order(false);

    let optimized = optimize(
        &plan,
        &empty_query(),
        &shop_model(),
        &shop_schema(),
        Some(&state),
        &anyone(),
    );

    assert_eq!(optimized.plan(), &plan);
}

#[test]
fn e5_2_3_the_order_of_the_state_keys_does_not_change_the_output() {
    let plan = filtered_scan();
    let (model, schema, query, security) = (shop_model(), shop_schema(), empty_query(), anyone());
    let first = state_in_order(false);
    let second = state_in_order(true);

    let from_first = optimize(&plan, &query, &model, &schema, Some(&first), &security);
    let from_second = optimize(&plan, &query, &model, &schema, Some(&second), &security);

    assert_eq!(from_first, from_second);
}

#[test]
fn e5_3_7_a_row_policy_filter_is_kept() {
    let plan = filtered_scan();

    let optimized = optimize(
        &plan,
        &empty_query(),
        &shop_model(),
        &shop_schema(),
        None,
        &anyone(),
    );

    assert_eq!(optimized.into_plan(), plan);
}

/// An optimized `filtered_scan` whose only record considers a summary with `outcome`.
fn with_summary_record(outcome: crate::RewriteOutcome) -> crate::OptimizedPlan {
    crate::OptimizedPlan {
        plan: filtered_scan(),
        records: vec![crate::RewriteRecord {
            rule: RewriteRule::UseSummary,
            subject: crate::RewriteSubject::Summary(
                menura_model::internal_model::NormalizedName::new("ORDERS_MONTHLY"),
            ),
            outcome,
        }],
        explanation: Vec::new(),
    }
}

#[test]
fn sum_4_2_a_plan_that_uses_a_summary_requires_the_authorized_executor() {
    let optimized = with_summary_record(crate::RewriteOutcome::Applied);
    assert!(optimized.requires_authorized_executor());
}

#[test]
fn sum_4_2_a_plan_whose_summaries_were_all_rejected_needs_no_authorized_executor() {
    let optimized = with_summary_record(crate::RewriteOutcome::Rejected {
        failed_condition: "SUM-3.2".to_owned(),
    });
    assert!(!optimized.requires_authorized_executor());
}
