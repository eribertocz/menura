//! Rust types for every contract document, serialized exactly as its JSON Schema.
//!
//! Every object rejects unknown members, closed sets are enums, and alternatives the schema
//! makes exclusive are enums, so a decoded document cannot be in a state its schema forbids.

pub mod audit_record;
pub mod capabilities;
pub mod catalog;
pub mod common;
pub mod compiled_plan;
pub mod descriptions;
pub mod query;
pub mod security_context;

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::contract::ContractKind;

/// A Rust type that represents one contract document.
pub trait ContractDocument: Serialize + DeserializeOwned {
    /// The contract this type represents.
    const KIND: ContractKind;
}

macro_rules! contract_document {
    ($type:ty, $kind:ident) => {
        impl ContractDocument for $type {
            const KIND: ContractKind = ContractKind::$kind;
        }
    };
}

contract_document!(catalog::CatalogSchemaDocument, CatalogSchema);
contract_document!(catalog::CatalogStateDocument, CatalogState);
contract_document!(query::QueryDocument, Query);
contract_document!(security_context::SecurityContextDocument, SecurityContext);
contract_document!(compiled_plan::CompiledPlanDocument, CompiledPlan);
contract_document!(compiled_plan::CompileErrorDocument, CompileError);
contract_document!(audit_record::AuditRecordDocument, AuditRecord);
contract_document!(descriptions::ModelDescriptionDocument, ModelDescription);
contract_document!(descriptions::IntegrityChecksDocument, IntegrityChecks);
