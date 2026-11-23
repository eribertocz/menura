//! The fixed parameters of a generated benchmark model (R4, R5).

/// The fixed parameters from which a benchmark model is generated (R4).
///
/// A model has one fact dataset `sales` in a star with `star_datasets` dimension datasets and
/// at the head of a chain of `chain_datasets` datasets, all joined by many-to-one
/// relationships; a fact dataset `stock` for the semi-additive metrics; and two summaries of
/// `sales`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelShape {
    /// Name of the semantic model.
    pub name: &'static str,
    /// Dimension datasets in the star around `sales`.
    pub star_datasets: usize,
    /// Datasets in the chain that starts at `sales`.
    pub chain_datasets: usize,
    /// Fields of every dataset together.
    pub total_fields: usize,
    /// Base metrics on `sales`.
    pub base_metrics: usize,
    /// Derived metrics over two base metrics.
    pub derived_metrics: usize,
    /// Semi-additive metrics on `stock`.
    pub semi_additive_metrics: usize,
    /// Context metrics over a base metric.
    pub context_metrics: usize,
    /// Star datasets, from the first, that declare a row policy propagated to `sales`.
    pub row_policy_datasets: usize,
}

/// Datasets that are neither in the star nor in the chain: `sales`, `stock` and two summaries.
pub(crate) const OTHER_DATASETS: usize = 4;

impl ModelShape {
    /// The reference benchmark model of R5: 50 datasets, 500 fields, 100 metrics and row
    /// policies on 10 datasets.
    #[must_use]
    pub const fn reference() -> Self {
        Self {
            name: "benchmark",
            star_datasets: 30,
            chain_datasets: 16,
            total_fields: 500,
            base_metrics: 60,
            derived_metrics: 20,
            semi_additive_metrics: 10,
            context_metrics: 10,
            row_policy_datasets: 10,
        }
    }

    /// The R1 model: 500 datasets and 5,000 fields of the same kinds as the reference model.
    #[must_use]
    pub const fn r1() -> Self {
        Self {
            name: "benchmark_r1",
            star_datasets: 300,
            chain_datasets: 196,
            total_fields: 5000,
            base_metrics: 600,
            derived_metrics: 200,
            semi_additive_metrics: 100,
            context_metrics: 100,
            row_policy_datasets: 100,
        }
    }

    /// Every dataset of the model, summaries included.
    #[must_use]
    pub const fn datasets(&self) -> usize {
        self.star_datasets + self.chain_datasets + OTHER_DATASETS
    }

    /// Every metric of the model.
    #[must_use]
    pub const fn metrics(&self) -> usize {
        self.base_metrics + self.derived_metrics + self.semi_additive_metrics + self.context_metrics
    }
}
