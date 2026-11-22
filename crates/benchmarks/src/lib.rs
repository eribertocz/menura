//! The performance benchmarks of `spec/05-engineering-standards.md` section 5.6 (R1–R7).
//!
//! Test tooling, like `conformance-runner` (R4, ARC-5.3): never shipped, no other crate
//! depends on it, and it calls only the public functions of `core`. Its models and queries
//! are generated from the fixed parameters of [`ModelShape`], so every run reads the same
//! inputs. It measures wall-clock time (R6) and counts instructions with Valgrind Callgrind
//! run as an external tool (R7).
//!
//! # Example
//!
//! ```
//! use menura_benchmarks::{ModelShape, model_document};
//!
//! let shape = ModelShape::reference();
//! assert_eq!(model_document(&shape), model_document(&shape));
//! assert_eq!(shape.datasets(), 50);
//! ```

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "R4, R6, R7: test tooling like the conformance runner; it measures wall-clock time, \
              reads Callgrind output files and runs Valgrind"
)]

mod baseline;
mod callgrind;
mod catalog_document;
mod datasets;
mod error;
mod inputs;
mod metrics;
mod model_document;
mod model_shape;
mod queries;
mod report;
mod scenarios;
mod targets;
mod wall_clock;

pub use baseline::{InstructionCounts, Regression};
pub use callgrind::{count_compilation, total_instructions};
pub use catalog_document::catalog_schema_document;
pub use error::BenchmarkError;
pub use inputs::{ModelInputs, compile_query};
pub use model_document::model_document;
pub use model_shape::ModelShape;
pub use queries::{
    BenchmarkQuery, SWEEP_ITEMS, reference_queries, security_context_document, sweep_query,
};
pub use report::{Report, instruction_report, wall_clock_report};
pub use scenarios::{run_scenario, scenario_queries};
pub use targets::{R1_LOAD_LIMIT, R2_COMPILE_LIMIT, r1_passes, r2_passes, r3_violations};
pub use wall_clock::{
    MEASURED_COMPILATIONS, WARM_UP_COMPILATIONS, percentile_99, time_compile, time_load,
};
