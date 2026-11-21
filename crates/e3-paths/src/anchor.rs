//! The anchor of a dimension-only query (E3-5.1, E3-5.2, E2-5 rows 15 to 17).

use menura_model::internal_model::NormalizedName;
use menura_representations::origin::QueryLocation;

use crate::error::{PathCode, PathDiagnostic};
use crate::needed::NeededDatasets;
use crate::reachability::AuthorizedGraph;

/// The pointer of `query.dimensions`, where anchor errors are located (E3-5.1).
const DIMENSIONS: &str = "/query/dimensions";

/// Chooses the anchor of the dimension-only query `needed`: the dimension dataset from which
/// every other dimension dataset is reachable (E3-5.1).
///
/// # Errors
///
/// `E_NO_ANCHOR` at `/query/dimensions` when no dimension dataset is an anchor; then
/// `E_UNSUPPORTED_FILTER_PATH` at the first filter, or `date_range` entry, whose dataset an
/// anchor does not reach (E3-5.2); then `E_AMBIGUOUS_PATH` at `/query/dimensions` when more
/// than one dimension dataset is an anchor.
pub(crate) fn choose_anchor<'input>(
    needed: &NeededDatasets<'input>,
    graph: &AuthorizedGraph<'_>,
) -> Result<&'input NormalizedName, PathDiagnostic> {
    let mut datasets: Vec<&NormalizedName> = Vec::new();
    for dimension in &needed.dimensions {
        if !datasets.contains(&dimension.dataset) {
            datasets.push(dimension.dataset);
        }
    }
    let anchors: Vec<&NormalizedName> = datasets
        .iter()
        .copied()
        .filter(|candidate| {
            datasets
                .iter()
                .all(|dataset| graph.reaches(candidate, dataset))
        })
        .collect();
    let location = QueryLocation::new(DIMENSIONS, None);
    let Some(anchor) = anchors.first().copied() else {
        return Err(PathDiagnostic::new(
            PathCode::NoAnchor,
            "no dataset of the dimensions reaches the datasets of all the others through the \
             relationships you may use; query dimensions of datasets that one of them reaches"
                .to_owned(),
            &location,
        ));
    };
    if let Some(element) = needed.filters.iter().find(|filter| {
        !anchors
            .iter()
            .all(|candidate| graph.reaches(candidate, filter.dataset))
    }) {
        return Err(PathDiagnostic::unsupported_filter_path(
            element.dataset,
            element.origin,
            "the dataset the rows come from",
        ));
    }
    if anchors.len() > 1 {
        return Err(PathDiagnostic::new(
            PathCode::AmbiguousPath,
            "more than one dataset of the dimensions reaches the datasets of all the others, so \
             the rows could come from any of them; add a metric or remove a dimension"
                .to_owned(),
            &location,
        ));
    }
    Ok(anchor)
}
