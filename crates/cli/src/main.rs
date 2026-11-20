//! Command-line wrapper around the core, without compilation logic of its own (A4).
//!
//! `menura validate` validates a model against a catalog schema (ARC-5.4, ARC-5.5).

#![forbid(unsafe_code)]

mod arguments;
mod run;

use std::process::ExitCode;

fn main() -> ExitCode {
    run::run()
}
