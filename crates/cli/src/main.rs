//! Command-line wrapper around the core, without compilation logic of its own (A4).
//!
//! `menura validate` validates a model against a catalog schema (ARC-5.4, ARC-5.5);
//! `menura compile`, `explain`, `describe` and `integrity-checks` write the documents of their
//! core functions (ARC-5.7, ARC-5.8); `menura migrate` writes the migration report (ARC-5.9);
//! `menura dependencies` and `menura diff` write the model-dependency document of a model and
//! the diff of two (ARC-5.6).

#![forbid(unsafe_code)]

mod arguments;
mod files;
mod run;
mod usage;

use std::process::ExitCode;

fn main() -> ExitCode {
    run::run()
}
