//! `conformance-clickhouse <conformance-directory> <server-url> <summary-file>`: runs every
//! `plan` and `runtime_error` case on the `ClickHouse` server at `server-url`
//! (`http://host:port`), prints the report per rule ID and the server facts, and writes the
//! summary the compatibility matrix reads (E6-7.10, E6-7.16, CI-4.3). Exits `1` when a case
//! fails and `2` when the run cannot start.

#![forbid(unsafe_code)]
#![allow(
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "A2: like the command-line wrapper, the ClickHouse run reads its arguments, \
              reaches the server and writes its summary"
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use menura_conformance_runner::clickhouse::{
    ClickhouseServer, ServerFacts, run_on_clickhouse, server_facts,
};
use menura_conformance_runner::{CaseStatus, Report};
use serde_json::json;

const USAGE: &str =
    "usage: conformance-clickhouse <conformance-directory> <server-url> <summary-file>";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [directory, url, summary] = arguments.as_slice() else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match run(Path::new(directory), url, &PathBuf::from(summary)) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(reason) => {
            eprintln!("{reason}");
            ExitCode::from(2)
        }
    }
}

/// Runs the cases and writes the summary; `true` when no case failed.
fn run(directory: &Path, url: &str, summary: &Path) -> Result<bool, String> {
    let server = ClickhouseServer::new(url)?;
    let facts = server_facts(&server).map_err(|error| error.to_string())?;
    let report = run_on_clickhouse(directory, &server).map_err(|error| error.to_string())?;
    print!("{}", report.render());
    println!("ClickHouse {}", facts.version);
    println!("ClickHouse time zone {}", facts.time_zone);
    println!("ClickHouse time-zone data {}", facts.time_zone_data_version);
    write_summary(summary, &report, &facts)?;
    Ok(!report.has_failures())
}

/// Writes the summary of the run as JSON (CI-4.3, E6-7.16).
fn write_summary(path: &Path, report: &Report, facts: &ServerFacts) -> Result<(), String> {
    let passed = report
        .cases
        .iter()
        .filter(|case| case.status == CaseStatus::Passed)
        .count();
    let summary = json!({
        "job": "conformance-clickhouse",
        "dialect": "clickhouse",
        "clickhouse_version": facts.version,
        "clickhouse_time_zone": facts.time_zone,
        "clickhouse_time_zone_data_version": facts.time_zone_data_version,
        "cases": {"passed": passed, "total": report.cases.len()},
    });
    let unwritable = |error: std::io::Error| format!("{}: {error}", path.display());
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder).map_err(unwritable)?;
    }
    let text = serde_json::to_string_pretty(&summary).map_err(|error| error.to_string())?;
    fs::write(path, format!("{text}\n")).map_err(unwritable)
}
