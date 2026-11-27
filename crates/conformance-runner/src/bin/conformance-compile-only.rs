//! `conformance-compile-only <conformance-directory>`: checks every conformance case as the
//! runner does, but executes no plan, so it needs no `DuckDB` (PLT-1.2). Every output a case
//! gives (SQL, plan, error, description and check documents) is compared with its golden files;
//! the report is the runner's (CS-4.3). Any failed case exits `1`.
//!
//! The `platforms` job builds it without the `duckdb` feature, on every platform of PLT-1.1.

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    reason = "A2: like the command-line wrapper, the compile-only check reads its arguments"
)]

use std::path::PathBuf;
use std::process::ExitCode;

use menura_conformance_runner::run_compile_only;

const USAGE: &str = "usage: conformance-compile-only <conformance-directory>";

fn main() -> ExitCode {
    let arguments: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    let [directory] = arguments.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run_compile_only(directory) {
        Ok(report) => {
            print!("{}", report.render());
            println!("compile-only: no plan was executed (PLT-1.2)");
            if report.has_failures() {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}
