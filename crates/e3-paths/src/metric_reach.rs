//! The reachability checks of a query with metrics (E3-3.4 to E3-3.6, E2-5 rows 13, 14
//! and 16).

use std::collections::BTreeMap;

use menura_model::internal_model::NormalizedName;

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
    let reach = MetricReach::new(needed, graph);
    reach.check_no_path()?;
    reach.check_partial_reachability()?;
    reach.check_filter_paths()
}

/// The metrics of a query and, for each dataset a dimension or a filter needs, how many of
/// them reach it.
struct MetricReach<'check, 'input> {
    needed: &'check NeededDatasets<'input>,
    /// For each needed dataset, how many metrics reach it (E3-3.4).
    reaching: BTreeMap<&'input NormalizedName, usize>,
}

impl<'check, 'input> MetricReach<'check, 'input> {
    /// Counts, for each distinct dataset of `needed`, the metrics that reach it through
    /// `graph`. Metrics with the same home datasets are checked once, and each needed dataset
    /// once, so the work grows with the distinct datasets, not with the number of items.
    fn new(needed: &'check NeededDatasets<'input>, graph: &AuthorizedGraph<'_>) -> Self {
        let metric_groups = metric_groups(needed);
        let mut reaching = BTreeMap::new();
        for element in needed.dimensions.iter().chain(&needed.filters) {
            reaching.entry(element.dataset).or_insert_with(|| {
                metric_groups
                    .iter()
                    .filter(|(homes, _)| graph.all_reach(homes, element.dataset))
                    .map(|(_, count)| count)
                    .sum()
            });
        }
        Self { needed, reaching }
    }

    /// How many metrics reach the dataset of `element` (E3-3.4).
    fn reaching(&self, element: &NeededDataset<'_>) -> usize {
        self.reaching
            .get(element.dataset)
            .copied()
            .unwrap_or_default()
    }

    /// Whether some metric does not reach the dataset of `element`.
    fn not_reached_by_all(&self, element: &NeededDataset<'_>) -> bool {
        self.reaching(element) < self.needed.metrics.len()
    }

    /// E2-5 row 13: `E_NO_PATH` at the first dimension no metric reaches (E3-3.5).
    fn check_no_path(&self) -> Result<(), PathDiagnostic> {
        match self
            .needed
            .dimensions
            .iter()
            .find(|dimension| self.reaching(dimension) == 0)
        {
            Some(element) => Err(PathDiagnostic::new(
                PathCode::NoPath,
                format!(
                    "no metric of the query reaches dataset `{}` of this dimension through the \
                     relationships you may use; remove the dimension or query a metric that \
                     reaches it",
                    element.dataset.as_str()
                ),
                element.origin,
            )),
            None => Ok(()),
        }
    }

    /// E2-5 row 14: `E_PARTIAL_DIMENSION_REACHABILITY` at the first dimension only some
    /// metrics reach (E3-3.5).
    fn check_partial_reachability(&self) -> Result<(), PathDiagnostic> {
        match self
            .needed
            .dimensions
            .iter()
            .find(|dimension| self.not_reached_by_all(dimension))
        {
            Some(element) => Err(PathDiagnostic::new(
                PathCode::PartialDimensionReachability,
                format!(
                    "only some metrics of the query reach dataset `{}` of this dimension; query \
                     the metrics that do not reach it separately",
                    element.dataset.as_str()
                ),
                element.origin,
            )),
            None => Ok(()),
        }
    }

    /// E2-5 row 16: `E_UNSUPPORTED_FILTER_PATH` at the first filter, or `date_range` entry,
    /// whose dataset some metric does not reach (E3-3.6).
    fn check_filter_paths(&self) -> Result<(), PathDiagnostic> {
        match self
            .needed
            .filters
            .iter()
            .find(|filter| self.not_reached_by_all(filter))
        {
            Some(element) => Err(PathDiagnostic::unsupported_filter_path(
                element.dataset,
                element.origin,
                "some metric of the query",
            )),
            None => Ok(()),
        }
    }
}

/// The metrics of `needed` grouped by their distinct home datasets, with how many metrics
/// each group holds. Whether a metric reaches a dataset depends only on that set (E3-3.4).
fn metric_groups<'input>(
    needed: &NeededDatasets<'input>,
) -> BTreeMap<Vec<&'input NormalizedName>, usize> {
    let mut groups = BTreeMap::new();
    for homes in &needed.metrics {
        let mut distinct = homes.clone();
        distinct.sort_unstable();
        distinct.dedup();
        *groups.entry(distinct).or_insert(0) += 1;
    }
    groups
}
