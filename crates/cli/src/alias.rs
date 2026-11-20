//! Command-line wrapper around the core, without compilation logic of its own (A4).
//!
//! `semantic-compiler` is the former name of `menura`, kept as an alias with the same
//! arguments and results until 1.0.0 (CON-0.5).

#![forbid(unsafe_code)]

mod arguments;
mod run;

use std::process::ExitCode;

fn main() -> ExitCode {
    run::run()
}
