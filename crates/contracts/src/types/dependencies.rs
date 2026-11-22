//! The model dependencies (DEP-2) and model diff (DEP-4) documents.

use serde::{Deserialize, Serialize};

use super::common::Producer;
use super::compiled_plan::PhysicalInput;
use super::descriptions::MetricKind;
use crate::contract::ContractVersion;
use crate::hash::Sha256Hash;

/// The `contract` value `menura.model_dependencies`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelDependenciesContract {
    /// The name the compiler writes.
    #[serde(rename = "menura.model_dependencies")]
    ModelDependencies,
}

/// The model-level dependency graph (DEP-1, DEP-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDependenciesDocument {
    /// `menura.model_dependencies`.
    pub contract: ModelDependenciesContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// The compiler build (PLN-1).
    pub producer: Producer,
    /// The model hash (E0H-3.2).
    pub model_hash: Sha256Hash,
    /// One entry per dataset, summaries included, ordered by `object` (DEP-3.2).
    pub datasets: Vec<ElementDependencies>,
    /// One entry per relationship, ordered by `object`.
    pub relationships: Vec<ElementDependencies>,
    /// One entry per field, ordered by `object`.
    pub fields: Vec<ElementDependencies>,
    /// One entry per model or dataset metric, ordered by `object`.
    pub metrics: Vec<MetricDependencies>,
    /// One entry per row policy, ordered by `object`.
    pub policies: Vec<ElementDependencies>,
}

/// The dependencies of a dataset, relationship, field or row policy (DEP-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ElementDependencies {
    /// `<kind>:<name>`, with normalized names.
    pub object: String,
    /// The element's definition hash (DEP-3.10).
    pub definition_hash: Sha256Hash,
    /// `false` for an element E0 marked unusable (E0-4.1).
    pub usable: bool,
    /// The code a query that reads the element gets; present only when `usable` is `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_error_code: Option<String>,
    /// Model elements it depends on, ordered as in PLN-4.4 with `dataset` first.
    pub model_inputs: Vec<String>,
    /// Catalog columns it depends on, ordered as in PLN-4.5.
    pub physical_inputs: Vec<PhysicalInput>,
    /// Row policies that restrict the rows it reads, ordered as in PLN-4.7.
    pub security_policies: Vec<String>,
}

/// The dependencies of a metric: those of every element, with its kind (DEP-2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetricDependencies {
    /// `metric:<name>` or `metric:<dataset>.<metric>`, with normalized names.
    pub object: String,
    /// `base`, `derived` or `context` (DSC-2).
    pub kind: MetricKind,
    /// The metric's definition hash (DEP-3.10).
    pub definition_hash: Sha256Hash,
    /// `false` for a metric E0 marked unusable (E0-4.1).
    pub usable: bool,
    /// The code a query that reads the metric gets; present only when `usable` is `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_error_code: Option<String>,
    /// Model elements it depends on, ordered as in PLN-4.4 with `dataset` first.
    pub model_inputs: Vec<String>,
    /// Catalog columns it depends on, ordered as in PLN-4.5.
    pub physical_inputs: Vec<PhysicalInput>,
    /// Row policies that restrict the rows it reads, ordered as in PLN-4.7.
    pub security_policies: Vec<String>,
}

/// The `contract` value `menura.model_diff`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelDiffContract {
    /// The name the compiler writes.
    #[serde(rename = "menura.model_diff")]
    ModelDiff,
}

/// The comparison of two model-dependency documents (DEP-4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelDiffDocument {
    /// `menura.model_diff`.
    pub contract: ModelDiffContract,
    /// Contract version.
    pub contract_version: ContractVersion,
    /// The compiler build.
    pub producer: Producer,
    /// The `model_hash` of the old document.
    pub old_model_hash: Sha256Hash,
    /// The `model_hash` of the new document.
    pub new_model_hash: Sha256Hash,
    /// The changed elements, ordered by kind, then by `object` (DEP-4.3).
    pub changes: Vec<ModelChange>,
}

/// One changed element (DEP-4.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelChange {
    /// The element, as `object` names it in the dependency documents.
    pub object: String,
    /// What happened to it.
    pub change: ChangeKind,
}

/// What happened to an element between two models (DEP-4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// Only in the new model.
    Added,
    /// Only in the old model.
    Removed,
    /// In both, with a different definition hash.
    Changed,
}
