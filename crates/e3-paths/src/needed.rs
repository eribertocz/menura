//! The datasets a query needs, in query order (E3-3.1a, E2-5.3), and the datasets of its
//! metrics (E3-3.4).

use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind, BoundQuery};
use menura_representations::origin::QueryLocation;
use menura_representations::paths::QueryElement;

use crate::components::components;
use crate::violation::InvariantViolation;

/// A dataset that a query element needs, with where the element is written.
#[derive(Debug, Clone)]
pub(crate) struct NeededDataset<'input> {
    /// The element, which the join tree binds to the instance at the end of its path
    /// (E3-4.2a).
    pub(crate) element: QueryElement,
    /// The dataset.
    pub(crate) dataset: &'input NormalizedName,
    /// Where the element that needs it is written.
    pub(crate) origin: &'input QueryLocation,
}

/// The needed datasets of a query and the datasets of its metrics.
#[derive(Debug, Default)]
pub(crate) struct NeededDatasets<'input> {
    /// The dataset of every dimension, in query order.
    pub(crate) dimensions: Vec<NeededDataset<'input>>,
    /// The datasets of `filter`, of the `filters` items and of the `date_range` dimension, in
    /// query order; a filter that reads no dataset adds none (E3-3.6).
    pub(crate) filters: Vec<NeededDataset<'input>>,
    /// For every metric, in query order, its components' home datasets (E3-3.4).
    pub(crate) metrics: Vec<Vec<&'input NormalizedName>>,
}

impl<'input> NeededDatasets<'input> {
    /// The needed datasets of `query`, reading the components of its metrics from `model`.
    ///
    /// # Errors
    ///
    /// The [`InvariantViolation`] of a metric whose components cannot be read from `model`.
    pub(crate) fn read(
        query: &'input BoundQuery,
        model: &'input InternalModel,
    ) -> Result<Self, InvariantViolation> {
        let dimensions = query
            .columns()
            .dimensions()
            .enumerate()
            .map(|(position, dimension)| dimension_dataset(position, dimension))
            .collect();
        let mut metrics = Vec::new();
        for metric in query.columns().metrics() {
            let mut homes: Vec<&NormalizedName> = components(metric, model)?
                .into_iter()
                .map(|component| &component.home_dataset)
                .collect();
            homes.sort();
            homes.dedup();
            metrics.push(homes);
        }
        Ok(Self {
            dimensions,
            filters: filter_datasets(query),
            metrics,
        })
    }
}

/// The dataset of `dimension`, the dimension at `position` among the dimensions.
fn dimension_dataset(position: usize, dimension: &BoundDimension) -> NeededDataset<'_> {
    let dataset = match &dimension.kind {
        BoundDimensionKind::Field { field, .. } => &field.dataset,
        BoundDimensionKind::Expression { dataset, .. } => dataset,
    };
    NeededDataset {
        element: QueryElement::Dimension(position),
        dataset,
        origin: &dimension.origin,
    }
}

/// The filter datasets of `query`, in query order: the filters, then the `date_range`, which
/// is located at its entry (E3-3.1a).
fn filter_datasets(query: &BoundQuery) -> Vec<NeededDataset<'_>> {
    let mut needed = Vec::new();
    for (position, filter) in query.filters().iter().enumerate() {
        for dataset in &filter.datasets {
            needed.push(NeededDataset {
                element: QueryElement::Filter {
                    filter: position,
                    dataset: dataset.clone(),
                },
                dataset,
                origin: &filter.origin,
            });
        }
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
