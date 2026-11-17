//! Runs the conformance suite of `spec/08-conformance-suite.md` and reports results per rule ID.
//!
//! The runner reads the case format of CS-2, prepares a fresh `DuckDB` database for every case as
//! CS-4.2 says, and reports each case under the rule IDs it covers (CS-4.3). No other crate may
//! depend on this one.
//!
//! # Example
//!
//! ```no_run
//! use std::path::Path;
//!
//! let report = semantic_compiler_conformance_runner::run(Path::new("conformance"), Some("sample"))?;
//! print!("{}", report.render());
//! # Ok::<(), semantic_compiler_conformance_runner::RunnerError>(())
//! ```

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: like the command-line wrapper, the conformance runner reads files and runs DuckDB"
)]

pub mod case;
pub mod case_file;
pub mod database;
pub mod error;
pub mod fixture;
pub mod load_options;
pub mod ossie;
pub mod report;
pub mod runner;

pub use error::{RunnerError, RunnerErrorKind};
pub use report::{CaseResult, CaseStatus, Report};
pub use runner::run;
