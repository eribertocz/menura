//! E5-4: the choice among valid summaries, and the records of the summaries considered.

use menura_model::internal_model::NormalizedName;

use super::super::rewrite;
use super::super::selection::{Candidate, smallest};
use super::{
    MONTHLY, ORDERS, by_region_and_month, deduplicate, model, query, scan, visible_orders,
    with_context,
};
use crate::record::{RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

fn candidate(summary: &str, row_count_estimate: u64) -> Candidate {
    Candidate {
        summary: NormalizedName::new(summary),
        row_count_estimate,
        branch: scan(ORDERS, &["AMOUNT"]),
    }
}

fn chosen(candidates: &[Candidate]) -> Option<&str> {
    smallest(candidates).map(|candidate| candidate.summary.as_str())
}

#[test]
fn e5_4_the_summary_with_the_smallest_row_count_estimate_is_chosen() {
    let candidates = [candidate("A", 30), candidate("B", 10), candidate("C", 20)];
    assert_eq!(chosen(&candidates), Some("B"));
}

#[test]
fn e5_4_equal_estimates_choose_the_first_summary_by_normalized_name() {
    let candidates = [candidate("ORDERS_B", 10), candidate("ORDERS_A", 10)];
    assert_eq!(chosen(&candidates), Some("ORDERS_A"));
}

#[test]
fn e5_4_no_candidate_chooses_no_summary() {
    assert_eq!(chosen(&[]), None);
}

#[test]
fn e5_2_1_without_a_catalog_state_the_summary_is_rejected_on_freshness() {
    let plan = by_region_and_month(visible_orders());
    let mut records = Vec::new();
    let rewritten = with_context(&model(), &query(), |context| {
        rewrite(plan.clone(), context, &mut records)
    });
    assert_eq!(rewritten, plan);
    assert_eq!(
        records,
        vec![RewriteRecord {
            rule: RewriteRule::UseSummary,
            subject: RewriteSubject::Summary(NormalizedName::new(MONTHLY)),
            outcome: RewriteOutcome::Rejected {
                failed_condition: "CAT-7.2 condition 1".to_owned(),
            },
        }]
    );
}

#[test]
fn e5_4_an_ineligible_summary_is_recorded_with_its_failed_condition() {
    let plan = by_region_and_month(deduplicate(visible_orders()));
    let mut records = Vec::new();
    let rewritten = with_context(&model(), &query(), |context| {
        rewrite(plan.clone(), context, &mut records)
    });
    assert_eq!(rewritten, plan);
    assert_eq!(
        records,
        vec![RewriteRecord {
            rule: RewriteRule::UseSummary,
            subject: RewriteSubject::Summary(NormalizedName::new(MONTHLY)),
            outcome: RewriteOutcome::Rejected {
                failed_condition: "SUM-3.2".to_owned(),
            },
        }]
    );
}
