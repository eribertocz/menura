//! The `alternatives` of an `E_NAME_NOT_FOUND` diagnostic (PLN-5.4): the authorized names
//! closest to the reference as written.

mod distance;

use distance::bounded_distance;

/// The kind of object a position of the query expects (PLN-5.4 step 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExpectedKind {
    /// A field, written `<dataset>.<field>`.
    Field,
    /// A metric.
    Metric,
    /// A dataset: a `paths` entry.
    Dataset,
}

impl ExpectedKind {
    /// The prefix of a written entry of this kind (PLN-5.4 step 4).
    pub(crate) fn prefix(self) -> &'static str {
        match self {
            Self::Field => "field:",
            Self::Metric => "metric:",
            Self::Dataset => "dataset:",
        }
    }
}

/// The largest Levenshtein distance a kept candidate may have (PLN-5.4 step 3).
const MAXIMUM_DISTANCE: usize = 2;

/// The longest written entry a kept candidate may have, in code points (PLN-5.4 step 3).
const MAXIMUM_ENTRY_LENGTH: usize = 256;

/// The most alternatives a diagnostic writes (PLN-5.4 step 4).
const MAXIMUM_ALTERNATIVES: usize = 5;

/// The alternatives of `reference`, the normalized reference as written, among `candidates`,
/// the normalized names of the authorized objects of `kind` (PLN-5.4 steps 3 and 4): each
/// within distance 2 and with a written entry of at most 256 code points, ordered by distance
/// then by name in CON-5 order, at most five, each written with the prefix of `kind`.
pub(crate) fn select_alternatives(
    kind: ExpectedKind,
    reference: &str,
    candidates: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let mut kept: Vec<(usize, String)> = candidates
        .into_iter()
        .map(|name| format!("{}{name}", kind.prefix()))
        .filter(|entry| entry.chars().count() <= MAXIMUM_ENTRY_LENGTH)
        .filter_map(|entry| {
            let name = &entry[kind.prefix().len()..];
            bounded_distance(name, reference, MAXIMUM_DISTANCE).map(|distance| (distance, entry))
        })
        .collect();
    kept.sort();
    kept.dedup();
    kept.into_iter()
        .take(MAXIMUM_ALTERNATIVES)
        .map(|(_, entry)| entry)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ExpectedKind, select_alternatives};

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn pln_5_4_orders_by_distance_then_name() {
        let candidates = names(&["SALES.SOLD_AT", "SALES.SALE_ID", "SALES.SALE_AT", "SALES.X"]);
        assert_eq!(
            select_alternatives(ExpectedKind::Field, "SALES.SALE_AR", candidates),
            names(&["field:SALES.SALE_AT", "field:SALES.SALE_ID"])
        );
    }

    #[test]
    fn pln_5_4_writes_at_most_five() {
        let candidates = names(&["AF", "AE", "AD", "AC", "AB", "AA"]);
        assert_eq!(
            select_alternatives(ExpectedKind::Metric, "A", candidates),
            names(&[
                "metric:AA",
                "metric:AB",
                "metric:AC",
                "metric:AD",
                "metric:AE"
            ])
        );
    }

    #[test]
    fn pln_5_4_drops_entries_longer_than_256_code_points() {
        let reference = "É".repeat(248);
        let too_long = "É".repeat(249);
        let kept = "É".repeat(247);
        let candidates = vec![too_long, kept.clone()];
        assert_eq!(
            select_alternatives(ExpectedKind::Dataset, &reference, candidates),
            vec![format!("dataset:{kept}")]
        );
    }

    #[test]
    fn pln_5_4_empty_without_candidates() {
        assert!(select_alternatives(ExpectedKind::Field, "S.F", Vec::new()).is_empty());
    }
}
