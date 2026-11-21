//! The reachability checks of a query with metrics (E3-3.4 to E3-3.6, E2-5 rows 13, 14
//! and 16).

use crate::error::{PathCode, PathDiagnostic};
use crate::needed::{NeededDataset, NeededDatasets};
use crate::reachability::AuthorizedGraph;

/// Checks that every metric of `needed` reaches every dimension dataset, then every filter
/// dataset. Each row of E2-5 is checked over every element before the next row (E2-5.3).
///
/// # Errors
///
/// `E_NO_PATH` at the first dimension no metric reaches, then
/// `E_PARTIAL_DIMENSION_REACHABILITY` at the first dimension only some metrics reach, then
/// `E_UNSUPPORTED_FILTER_PATH` at the first filter, or `date_range` entry, whose dataset some
/// metric does not reach.
pub(crate) fn check_metric_reach(
    needed: &NeededDatasets<'_>,
    graph: &AuthorizedGraph<'_>,
) -> Result<(), PathDiagnostic> {
    let reaching = |element: &NeededDataset<'_>| {
        needed
            .metrics
            .iter()
            .filter(|homes| graph.all_reach(homes, element.dataset))
            .count()
    };
    let metric_count = needed.metrics.len();
    if let Some(element) = needed
        .dimensions
        .iter()
        .find(|dimension| reaching(dimension) == 0)
    {
        return Err(PathDiagnostic::new(
            PathCode::NoPath,
            format!(
                "no metric of the query reaches dataset `{}` of this dimension through the \
                 relationships you may use; remove the dimension or query a metric that \
                 reaches it",
                element.dataset.as_str()
            ),
            element.origin,
        ));
    }
    if let Some(element) = needed
        .dimensions
        .iter()
        .find(|dimension| reaching(dimension) < metric_count)
    {
        return Err(PathDiagnostic::new(
            PathCode::PartialDimensionReachability,
            format!(
                "only some metrics of the query reach dataset `{}` of this dimension; query \
                 the metrics that do not reach it separately",
                element.dataset.as_str()
            ),
            element.origin,
        ));
    }
    match needed
        .filters
        .iter()
        .find(|filter| reaching(filter) < metric_count)
    {
        Some(element) => Err(unsupported_filter_path(element, "some metric of the query")),
        None => Ok(()),
    }
}

/// `E_UNSUPPORTED_FILTER_PATH` for the filter or `date_range` entry `element`, which `source`
/// does not reach (E3-3.6, E3-5.2).
pub(crate) fn unsupported_filter_path(element: &NeededDataset<'_>, source: &str) -> PathDiagnostic {
    PathDiagnostic::new(
        PathCode::UnsupportedFilterPath,
        format!(
            "{source} does not reach dataset `{}`, which this filter reads, through the \
             relationships you may use; remove the filter or filter on a dataset it reaches",
            element.dataset.as_str()
        ),
        element.origin,
    )
}
