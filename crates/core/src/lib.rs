//! Pure, deterministic orchestration of the compilation stages.
//!
//! The public functions are those of architecture section 3.5 that exist in this version:
//! [`validate`], [`load`], [`with_state`], [`compile`], [`explain`], [`describe`],
//! [`integrity_checks`], [`dependencies`], [`diff`] and [`migrate`]; [`build_producer`] gives the producer every output of this build
//! writes. Their inputs are the contract documents as bytes; their outputs are
//! contract documents and the immutable [`Handle`] (ARC-5.1).
//!
//! # Example
//!
//! ```
//! use menura_core::{CompileErrorContract, LoadOptions, load};
//!
//! let error = load(b"version: \"0.1.1\"", b"{}", None, &LoadOptions::default()).unwrap_err();
//! assert_eq!(error.contract, CompileErrorContract::CompileError);
//! ```

#![forbid(unsafe_code)]

mod audit;
mod cache_key;
mod catalog_state;
mod compile;
mod compile_options;
mod dependencies;
mod describe;
mod dialect_name;
mod dialect_sql;
mod explain;
mod explanation;
mod handle;
mod integrity_checks;
mod json;
mod json_form;
mod lineage;
mod load;
mod load_options;
mod migrate;
mod ordered_text;
mod passed_document;
mod plan_document;
mod producer;
mod stop;
mod usage;

pub use compile::{Compilation, CompileResult, compile};
pub use compile_options::{CompileOptions, CompileOptionsError};
pub use dependencies::{dependencies, diff};
pub use describe::{DescribeResult, describe};
pub use explain::explain;
pub use handle::Handle;
pub use integrity_checks::integrity_checks;
pub use json::{
    OutputError, audit_record_json, dependencies_json, description_json, diff_json, document_json,
    integrity_checks_json, migration_report_json, plan_json,
};
pub use load::{load, validate, with_state};
pub use load_options::{LoadOptions, LoadOptionsError};
pub use menura_contracts::types::audit_record::{AuditRecordDocument, OssieFormat};
pub use menura_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
pub use menura_contracts::types::common::Dialect;
pub use menura_contracts::types::compiled_plan::{
    CompileErrorContract, CompileErrorDocument, CompiledPlanDocument, Diagnostic,
    DiagnosticLocation, InputDocument,
};
pub use menura_contracts::types::dependencies::{
    ChangeKind, ElementDependencies, MetricDependencies, ModelChange, ModelDependenciesDocument,
    ModelDiffDocument,
};
pub use menura_contracts::types::descriptions::{
    AiContext, AmbiguousPath, DescribedDataset, DescribedField, DescribedMetric,
    IntegrityChecksDocument, MetricKind, ModelDescriptionDocument,
};
pub use menura_contracts::types::migration_report::{
    MigrationChange, MigrationReportDocument, MigrationRule,
};
pub use menura_model::internal_model::InternalModel;
pub use migrate::migrate;
pub use producer::build_producer;
pub use usage::{
    Argument, CommandName, UsageError, UsageReason, check_audit_record, parse_compile_options,
    parse_dialect, parse_load_options,
};
