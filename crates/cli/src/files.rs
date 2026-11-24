//! The files and values each command of `menura` reads (ARC-5.5 to ARC-5.9).

use std::path::PathBuf;

use menura_core::Dialect;

/// The files of `validate` and `dependencies`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ModelFiles {
    /// `--model`.
    pub(crate) model: PathBuf,
    /// `--catalog-schema`.
    pub(crate) catalog_schema: PathBuf,
    /// `--load-options`, when given.
    pub(crate) load_options: Option<PathBuf>,
}

/// The files of `diff`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiffFiles {
    /// `--old-model`.
    pub(crate) old_model: PathBuf,
    /// `--old-catalog-schema`.
    pub(crate) old_catalog_schema: PathBuf,
    /// `--new-model`.
    pub(crate) new_model: PathBuf,
    /// `--new-catalog-schema`.
    pub(crate) new_catalog_schema: PathBuf,
    /// `--load-options`, when given.
    pub(crate) load_options: Option<PathBuf>,
}

/// The files of `compile` and `explain` (ARC-5.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompileFiles {
    /// `--model`, `--catalog-schema`, `--catalog-state` and `--load-options`.
    pub(crate) handle: HandleFiles,
    /// `--query`.
    pub(crate) query: PathBuf,
    /// `--security-context`.
    pub(crate) security_context: PathBuf,
    /// `--compile-options`, when given.
    pub(crate) compile_options: Option<PathBuf>,
    /// `--audit-record`, when given; only `compile` takes it.
    pub(crate) audit_record: Option<PathBuf>,
}

/// The files of `describe` (ARC-5.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DescribeFiles {
    /// `--model`, `--catalog-schema`, `--catalog-state` and `--load-options`.
    pub(crate) handle: HandleFiles,
    /// `--security-context`.
    pub(crate) security_context: PathBuf,
}

/// The files that load a handle: the model, its catalog schema, an optional catalog state and
/// optional load options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HandleFiles {
    /// `--model`.
    pub(crate) model: PathBuf,
    /// `--catalog-schema`.
    pub(crate) catalog_schema: PathBuf,
    /// `--catalog-state`, when given.
    pub(crate) catalog_state: Option<PathBuf>,
    /// `--load-options`, when given.
    pub(crate) load_options: Option<PathBuf>,
}

/// The files and dialect of `integrity-checks` (ARC-5.8).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IntegrityChecksFiles {
    /// `--model`, `--catalog-schema` and `--load-options`.
    pub(crate) model: ModelFiles,
    /// `--dialect`: `clickhouse` or `duckdb`; `duckdb` by default.
    pub(crate) dialect: Dialect,
}

/// The files of `migrate` (ARC-5.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigrateFiles {
    /// `--model`.
    pub(crate) model: PathBuf,
    /// `--load-options`, when given.
    pub(crate) load_options: Option<PathBuf>,
}
