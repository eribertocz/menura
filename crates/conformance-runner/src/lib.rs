//! Runs the conformance suite of `spec/08-conformance-suite.md` and reports results per rule ID.
//!
//! The runner reads the case format of CS-2, prepares a fresh `DuckDB` database for every case as
//! CS-4.2 says, runs the load part and the query part of every case through `core` (CS-2.8),
//! executes every plan on that database (E6-5.4), and reports each case under the rule IDs it
//! covers (CS-4.3). A differential case also compares the rows of its plan with those of its
//! oracle (CS-4.7). No other crate may depend on this one.
//!
//! Built without the default feature `duckdb`, the crate links no `DuckDB`: [`run_compile_only`]
//! then checks every case against its golden files without executing any plan (PLT-1.2).
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//!
//! let report = menura_conformance_runner::run(Path::new("conformance"), Some("sample"))?;
//! print!("{}", report.render());
//! # Ok::<(), menura_conformance_runner::RunnerError>(())
//! ```

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: like the command-line wrapper, the conformance runner reads files and runs DuckDB"
)]

mod alternatives_check;
mod audit_check;
#[cfg(feature = "duckdb")]
mod calendar;
mod canonical_json;
pub mod case;
pub mod case_file;
#[cfg(feature = "duckdb")]
mod cells;
#[cfg(feature = "duckdb")]
pub mod clickhouse;
mod clickhouse_literals;
mod compile_options;
#[cfg(feature = "duckdb")]
mod csv_rows;
#[cfg(feature = "duckdb")]
pub mod database;
mod describe_part;
pub mod determinism;
mod diagnostic_check;
#[cfg(feature = "duckdb")]
mod differential;
#[cfg(feature = "duckdb")]
mod divergences;
pub mod error;
#[cfg(feature = "duckdb")]
mod execution;
mod expectation_rules;
mod explain_part;
pub mod fixture;
mod golden_json;
mod inlined_values;
mod integrity_checks_part;
mod intended_literals;
pub mod load_options;
mod load_part;
#[cfg(feature = "duckdb")]
pub mod ossie;
#[cfg(feature = "duckdb")]
mod parameter_values;
#[cfg(feature = "duckdb")]
mod plan_execution;
mod plan_outputs;
mod query_part;
pub mod regenerate;
pub mod report;
#[cfg(feature = "duckdb")]
mod row_comparison;
pub mod runner;
mod schema_versions;
mod sql_literals;
pub mod wrappers;

pub use error::{RunnerError, RunnerErrorKind};
pub use regenerate::{Regeneration, regenerate};
pub use report::{CaseResult, CaseStatus, Report};
#[cfg(feature = "duckdb")]
pub use runner::run;
pub use runner::run_compile_only;
