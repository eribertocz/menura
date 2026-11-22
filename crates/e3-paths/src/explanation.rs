//! The decisions E3 records in the explanation (PLN-7.3): the `paths` entries of the query it
//! accepted (E3-3.3).

use menura_contracts::types::common::Step;
use menura_model::explanation::{ExplanationEntry, Subject};
use menura_representations::bound_query::BoundQuery;

/// The entries of E3 for `query`, whose paths [`crate::resolve_paths`] accepted: one `E3-3.3`
/// entry per `paths` entry, in query order. A `paths` entry that E3 does not accept fails
/// compilation (`E3-01`), so every entry of a compiled query is accepted.
#[must_use]
pub fn explanation(query: &BoundQuery) -> Vec<ExplanationEntry> {
    query
        .paths()
        .iter()
        .map(|path| ExplanationEntry {
            code: "E3-3.3",
            step: Step::E3,
            subject: Subject::Path {
                from: path.from().clone(),
                to: path.to().clone(),
            },
            decision: "path chosen by the query".to_owned(),
            reason: "the query names the path, which chains correctly from a home dataset to a \
                     dataset a branch needs"
                .to_owned(),
        })
        .collect()
}
