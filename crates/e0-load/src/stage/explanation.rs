//! The decision E0 records in the explanation of every plan: how datasets without `access`
//! are read (DEV-4, PLN-7.3).

use menura_contracts::types::audit_record::LoadOptions;
use menura_contracts::types::common::Step;
use menura_model::explanation::{ExplanationEntry, Subject};

/// The entries of E0: the `DEV-4` decision of the load option `grant_authenticated_to_all`.
pub(super) fn explanation(options: &LoadOptions) -> Vec<ExplanationEntry> {
    let (decision, value) = if options.grant_authenticated_to_all {
        ("datasets without access are read as authenticated", "true")
    } else {
        ("datasets without access are inaccessible", "false")
    };
    vec![ExplanationEntry {
        code: "DEV-4",
        step: Step::E0,
        subject: Subject::LoadOption("grant_authenticated_to_all"),
        decision: decision.to_owned(),
        reason: format!("grant_authenticated_to_all is {value}"),
    }]
}
