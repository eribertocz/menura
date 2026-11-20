//! The one dataset that a query of the 0.3.0 slice reads (VS-1).

use menura_model::internal_model::NormalizedName;
use menura_representations::origin::QueryLocation;

use crate::violation::InvariantViolation;

/// The datasets read so far, which must all be the same one.
#[derive(Debug, Default)]
pub(crate) struct SingleDataset {
    first: Option<NormalizedName>,
}

impl SingleDataset {
    /// Records that the element written at `origin` reads `dataset`.
    ///
    /// # Errors
    ///
    /// [`InvariantViolation::SeveralDatasets`] when `dataset` is not the first dataset read.
    pub(crate) fn read(
        &mut self,
        dataset: &NormalizedName,
        origin: &QueryLocation,
    ) -> Result<(), InvariantViolation> {
        match &self.first {
            None => {
                self.first = Some(dataset.clone());
                Ok(())
            }
            Some(first) if first == dataset => Ok(()),
            Some(first) => Err(InvariantViolation::SeveralDatasets {
                first: first.clone(),
                second: dataset.clone(),
                origin: origin.clone(),
            }),
        }
    }

    /// The dataset read, when any element read one.
    pub(crate) fn into_dataset(self) -> Option<NormalizedName> {
        self.first
    }
}
