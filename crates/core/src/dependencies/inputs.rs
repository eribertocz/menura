//! What one element depends on: model elements, catalog columns and row policies (DEP-2).

use std::collections::BTreeSet;

use menura_contracts::types::compiled_plan::PhysicalInput;

use super::names::{ModelInput, PolicyName};

/// A catalog column: its relation's `source`, then the column as the catalog spells it,
/// ordered as PLN-4.5 orders them.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CatalogColumn {
    /// The relation's catalog `source`.
    pub(super) source: String,
    /// The column, in the catalog's spelling.
    pub(super) column: String,
}

/// The dependencies of one element, each kind without duplicates and in contract order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Inputs {
    /// Model elements (DEP-3.4 to DEP-3.8).
    pub(super) model: BTreeSet<ModelInput>,
    /// Catalog columns.
    pub(super) physical: BTreeSet<CatalogColumn>,
    /// Row policies that restrict the rows read.
    pub(super) policies: BTreeSet<PolicyName>,
}

impl Inputs {
    /// Adds every dependency of `other`.
    pub(super) fn absorb(&mut self, other: &Self) {
        self.model.extend(other.model.iter().cloned());
        self.physical.extend(other.physical.iter().cloned());
        self.policies.extend(other.policies.iter().cloned());
    }

    /// `model_inputs` as written, without `itself` (DEP-2).
    pub(super) fn written_model_inputs(&self, itself: &ModelInput) -> Vec<String> {
        self.model
            .iter()
            .filter(|input| *input != itself)
            .map(ModelInput::written)
            .collect()
    }

    /// `physical_inputs` as written (PLN-4.5).
    pub(super) fn written_physical_inputs(&self) -> Vec<PhysicalInput> {
        self.physical
            .iter()
            .map(|column| PhysicalInput {
                source: column.source.clone(),
                column: column.column.clone(),
            })
            .collect()
    }

    /// `security_policies` as written (PLN-4.7).
    pub(super) fn written_policies(&self) -> Vec<String> {
        self.policies.iter().map(PolicyName::written).collect()
    }
}
