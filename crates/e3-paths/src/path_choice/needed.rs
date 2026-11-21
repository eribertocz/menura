//! The query elements that need a dataset, in query order (E3-3.1a, E2-5.3).

use menura_model::internal_model::NormalizedName;
use menura_representations::bound_query::{BoundDimensionKind, BoundQuery};
use menura_representations::columns::OutputColumn;
use menura_representations::origin::QueryLocation;
use menura_representations::paths::QueryElement;

/// One query element and one dataset it needs.
#[derive(Debug)]
pub(crate) struct NeededDataset<'query> {
    /// The element.
    pub(crate) element: QueryElement,
    /// The dataset it reads.
    pub(crate) dataset: &'query NormalizedName,
    /// Where the element is written, which locates its errors.
    pub(crate) origin: &'query QueryLocation,
}

/// The datasets every branch of `query` needs, by element, in query order: the dimensions,
/// then each filter with each dataset it reads, then the `date_range` (E3-3.1a). A filter
/// that reads no dataset needs none.
pub(crate) fn needed_datasets(query: &BoundQuery) -> Vec<NeededDataset<'_>> {
    let dimensions = query.columns().iter().filter_map(|column| match column {
        OutputColumn::Dimension(dimension) => Some(dimension),
        OutputColumn::Metric(_) => None,
    });
    let mut needed: Vec<NeededDataset<'_>> = dimensions
        .enumerate()
        .map(|(position, dimension)| NeededDataset {
            element: QueryElement::Dimension(position),
            dataset: match &dimension.kind {
                BoundDimensionKind::Field { field, .. } => &field.dataset,
                BoundDimensionKind::Expression { dataset, .. } => dataset,
            },
            origin: &dimension.origin,
        })
        .collect();
    for (position, filter) in query.filters().iter().enumerate() {
        needed.extend(filter.datasets.iter().map(|dataset| NeededDataset {
            element: QueryElement::Filter {
                filter: position,
                dataset: dataset.clone(),
            },
            dataset,
            origin: &filter.origin,
        }));
    }
    if let Some(date_range) = query.date_range() {
        needed.push(NeededDataset {
            element: QueryElement::DateRange,
            dataset: &date_range.field.dataset,
            origin: &date_range.origin,
        });
    }
    needed
}
