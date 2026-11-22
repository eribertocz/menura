//! The E5 entries of the explanation (E5-2.2, E5-3.8, PLN-7.3).

use menura_contracts::types::common::Step;
use menura_model::explanation::Subject;
use menura_model::internal_model::NormalizedName;

use crate::explanation::{authorized_executor_entries, rewrite_entries};
use crate::test_support::filtered_scan;
use crate::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};

fn summary_record(outcome: RewriteOutcome) -> RewriteRecord {
    RewriteRecord {
        rule: RewriteRule::UseSummary,
        subject: RewriteSubject::Summary(NormalizedName::new("ORDERS_MONTHLY")),
        outcome,
    }
}

#[test]
fn e5_3_8_a_node_record_names_the_home_dataset_of_its_branch() {
    let record = RewriteRecord {
        rule: RewriteRule::FoldConstants,
        subject: RewriteSubject::Node(NodePath::new(Vec::new())),
        outcome: RewriteOutcome::Applied,
    };

    let entries = rewrite_entries(&[record], &filtered_scan());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].code, "E5-3.6");
    assert_eq!(entries[0].step, Step::E5);
    assert_eq!(
        entries[0].subject,
        Subject::Dataset(NormalizedName::new("ORDERS"))
    );
    assert_eq!(entries[0].decision, "applied");
}

#[test]
fn e5_3_8_a_rejected_rewrite_names_its_failed_condition() {
    let record = summary_record(RewriteOutcome::Rejected {
        failed_condition: "SUM-3.4".to_owned(),
    });

    let entries = rewrite_entries(&[record], &filtered_scan());

    assert_eq!(
        entries[0].subject,
        Subject::Summary(NormalizedName::new("ORDERS_MONTHLY"))
    );
    assert_eq!(entries[0].decision, "rejected");
    assert!(entries[0].reason.contains("SUM-3.4"));
}

#[test]
fn e5_3_8_a_record_whose_node_the_plan_does_not_have_gets_no_entry() {
    let record = RewriteRecord {
        rule: RewriteRule::SkipDeduplication,
        subject: RewriteSubject::Node(NodePath::new(vec![0, 3])),
        outcome: RewriteOutcome::Applied,
    };

    assert!(rewrite_entries(&[record], &filtered_scan()).is_empty());
}

#[test]
fn e5_2_2_a_used_summary_requires_the_authorized_executor() {
    let records = [
        summary_record(RewriteOutcome::Rejected {
            failed_condition: "SUM-3.4".to_owned(),
        }),
        summary_record(RewriteOutcome::Applied),
    ];

    let entries = authorized_executor_entries(&records);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].code, "E5-4");
    assert_eq!(entries[0].decision, "requires authorized executor");
    assert_eq!(
        entries[0].subject,
        Subject::Summary(NormalizedName::new("ORDERS_MONTHLY"))
    );
}
