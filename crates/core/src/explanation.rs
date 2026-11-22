//! The explanation of the compiled plan: the entries every stage recorded, in step order
//! (PLN-7, PLN-7.1).

use std::collections::BTreeSet;

use menura_contracts::types::common::Decision;
use menura_model::explanation::ExplanationEntry;

/// The explanation of the stage entries `entries`, given in the order the stages ran: ordered
/// by `step` (`E0` first), then in emission order, each entry written once for its `code` and
/// `subject` (PLN-7.1).
pub(crate) fn assemble(entries: &[ExplanationEntry]) -> Vec<Decision> {
    let mut ordered: Vec<&ExplanationEntry> = entries.iter().collect();
    ordered.sort_by_key(|entry| entry.step as u8);
    let mut written: BTreeSet<(Option<String>, Option<String>)> = BTreeSet::new();
    let mut decisions = Vec::with_capacity(ordered.len());
    for entry in ordered {
        let decision = entry.to_decision();
        if written.insert((decision.code.clone(), decision.subject.clone())) {
            decisions.push(decision);
        }
    }
    decisions
}
