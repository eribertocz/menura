//! Runs every `plan` and `runtime_error` case on a `ClickHouse` server (E6-7.10, CS-2.3a), and
//! the `temporal_orphans` checks of every `integrity_checks` case (VAL-2.16a), whose rows must
//! be those `DuckDB` returns.
//!
//! For each case the runner prepares the case's `DuckDB` database as CS-4.2 says, loads its
//! model through `core` (CS-2.8), opens a fresh `ClickHouse` session whose settings of E6-7.8
//! are the opposite of those the generated SQL needs, copies the fixture's relations into that
//! session, checks that every copied column reads back with the type E6-7.13 maps to the
//! catalog schema's, compiles the query with `dialect: clickhouse`, and checks the plan. The
//! server's own time zone must not be UTC (E6-7.10).

mod case_check;
mod check_parity;
mod copy_check;
mod http;
mod parameters;
mod results;
mod server;
mod settings;
mod tables;
mod type_mapping;

use std::path::Path;

use duckdb::Connection;
use menura_core::Handle;

pub use server::ClickhouseServer;

use crate::case::{Case, Outcome};
use crate::case_file::read_case;
use crate::database::prepare_database;
use crate::error::{RunnerError, RunnerErrorKind};
use crate::fixture::{ModelFixture, read_bytes, security_context_path};
use crate::load_options::resolve_load_options;
use crate::load_part::{LoadOutcome, ModelDocuments, check_load};
use crate::report::{CaseResult, CaseStatus, Report};
use crate::runner::{area_folders, case_files};
use case_check::{ClickhouseInputs, check_on_clickhouse};
use check_parity::{check_parity, compared_checks};
use copy_check::copy_mismatch;
use tables::{drop_fixture, load_fixture};

/// What the compatibility matrix records about the server (CI-4.3, E6-7.16).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerFacts {
    /// The exact server version, as `version()` returns it.
    pub version: String,
    /// The server time zone, as `serverTimezone()` returns it.
    pub time_zone: String,
    /// The version of the server's embedded time-zone data (`TZDATA_VERSION`).
    pub time_zone_data_version: String,
}

/// Reads the facts of `server` and checks that its time zone is not UTC (E6-7.10).
///
/// # Errors
///
/// [`RunnerErrorKind::ClickhouseServer`] when the server cannot be queried or its time zone is
/// UTC.
pub fn server_facts(server: &ClickhouseServer) -> Result<ServerFacts, RunnerError> {
    let failed = |message: String| {
        RunnerError::new(RunnerErrorKind::ClickhouseServer(message), Path::new(""))
    };
    let read = |statement: &str| {
        server
            .query(statement)
            .map(|text| text.trim_end().to_owned())
            .map_err(|failure| failed(failure.describe()))
    };
    let facts = ServerFacts {
        version: read("SELECT version()")?,
        time_zone: read("SELECT serverTimezone()")?,
        time_zone_data_version: read(
            "SELECT value FROM system.build_options WHERE name = 'TZDATA_VERSION'",
        )?,
    };
    if is_utc(&facts.time_zone) {
        return Err(failed(format!(
            "the server time zone is `{}`; E6-7.10 requires a time zone other than UTC",
            facts.time_zone
        )));
    }
    if facts.time_zone_data_version.is_empty() {
        return Err(failed(
            "the server reports no time-zone data version (E6-7.16)".to_owned(),
        ));
    }
    Ok(facts)
}

/// Whether the server time zone `name` is UTC, or names none, which E6-7.10 refuses.
fn is_utc(name: &str) -> bool {
    matches!(
        name,
        "" | "UTC" | "Etc/UTC" | "Etc/UCT" | "UCT" | "GMT" | "Etc/GMT" | "Etc/GMT0" | "Zulu"
    )
}

/// Runs every `plan` and `runtime_error` case of `conformance_directory/cases/` on `server`,
/// and every `integrity_checks` case whose model has a `temporal_orphans` check.
///
/// # Errors
///
/// A [`RunnerError`] when the layout cannot be read or a case file breaks CS-1 or CS-2.
pub fn run_on_clickhouse(
    conformance_directory: &Path,
    server: &ClickhouseServer,
) -> Result<Report, RunnerError> {
    let mut report = Report::default();
    for area_folder in area_folders(&conformance_directory.join("cases"), None)? {
        for case_path in case_files(&area_folder)? {
            let case = read_case(&case_path)?;
            let session_id = format!("menura-conformance-{}", report.cases.len());
            let status = match case.expect.outcome {
                Outcome::Plan | Outcome::RuntimeError => run_case(
                    conformance_directory,
                    &area_folder,
                    &case,
                    server,
                    session_id,
                ),
                Outcome::IntegrityChecks => {
                    match run_check_case(conformance_directory, &case, server, session_id) {
                        Some(status) => status,
                        None => continue,
                    }
                }
                Outcome::LoadError | Outcome::Error | Outcome::Describe => continue,
            };
            report.cases.push(CaseResult {
                id: case.id,
                rules: case.rules,
                status,
            });
        }
    }
    Ok(report)
}

/// Prepares and runs one case in the fresh session `session_id`.
fn run_case(
    conformance_directory: &Path,
    case_folder: &Path,
    case: &Case,
    server: &ClickhouseServer,
    session_id: String,
) -> CaseStatus {
    let model_folder = conformance_directory.join("models").join(&case.model);
    let prepared = match prepare_case(&model_folder, case) {
        Ok(prepared) => prepared,
        Err(status) => return status,
    };
    let session = match server.open_session(session_id) {
        Ok(session) => session,
        Err(failure) => return CaseStatus::Failed(failure.describe()),
    };
    let loaded = match load_fixture(&session, &model_folder, &prepared.duckdb) {
        Ok(loaded) => loaded,
        Err(reason) => {
            return CaseStatus::Failed(format!("the fixture cannot be loaded: {reason}"));
        }
    };
    let inputs = ClickhouseInputs {
        case_folder,
        security_context: &prepared.security_context,
        session: &session,
    };
    let status = match copy_mismatch(&session, &loaded.tables) {
        Ok(None) => with_unloaded_sources(
            check_on_clickhouse(case, &prepared.handle, &inputs),
            &loaded.not_created,
        ),
        Ok(Some(mismatch)) => CaseStatus::Failed(mismatch),
        Err(reason) => CaseStatus::Failed(format!("the copied tables cannot be checked: {reason}")),
    };
    match drop_fixture(&session, &loaded) {
        Ok(()) => status,
        Err(reason) => CaseStatus::Failed(format!("the fixture cannot be dropped: {reason}")),
    }
}

/// Runs the `temporal_orphans` checks of the `integrity_checks` case `case` on both engines in
/// the fresh session `session_id`; `None` when its model has none, so the case is not run here.
fn run_check_case(
    conformance_directory: &Path,
    case: &Case,
    server: &ClickhouseServer,
    session_id: String,
) -> Option<CaseStatus> {
    let model_folder = conformance_directory.join("models").join(&case.model);
    let (duckdb, handle) = match prepare_database_and_model(&model_folder, case) {
        Ok(prepared) => prepared,
        Err(status) => return Some(status),
    };
    let checks = compared_checks(&handle);
    if checks.is_empty() {
        return None;
    }
    let session = match server.open_session(session_id) {
        Ok(session) => session,
        Err(failure) => return Some(CaseStatus::Failed(failure.describe())),
    };
    let loaded = match load_fixture(&session, &model_folder, &duckdb) {
        Ok(loaded) => loaded,
        Err(reason) => {
            return Some(CaseStatus::Failed(format!(
                "the fixture cannot be loaded: {reason}"
            )));
        }
    };
    let status = match copy_mismatch(&session, &loaded.tables) {
        Ok(None) => with_unloaded_sources(
            check_parity(&handle, &checks, &duckdb, &session),
            &loaded.not_created,
        ),
        Ok(Some(mismatch)) => CaseStatus::Failed(mismatch),
        Err(reason) => CaseStatus::Failed(format!("the copied tables cannot be checked: {reason}")),
    };
    Some(match drop_fixture(&session, &loaded) {
        Ok(()) => status,
        Err(reason) => CaseStatus::Failed(format!("the fixture cannot be dropped: {reason}")),
    })
}

/// What a case runs with once its `DuckDB` database is prepared and its model is loaded.
struct PreparedCase {
    /// The case's `DuckDB` database (CS-4.2), the source of the rows copied to `ClickHouse`.
    duckdb: Connection,
    /// The loaded model.
    handle: Box<Handle>,
    /// The security context document, as bytes.
    security_context: Vec<u8>,
}

/// Prepares the `DuckDB` database of `case`, loads its model and reads its security context,
/// or returns the status that decides the case: a load that fails or a fixture that cannot be
/// read.
fn prepare_case(model_folder: &Path, case: &Case) -> Result<PreparedCase, CaseStatus> {
    let failed = |error: RunnerError| CaseStatus::Failed(error.to_string());
    let (duckdb, handle) = prepare_database_and_model(model_folder, case)?;
    let context_name = case
        .security_context
        .as_deref()
        .ok_or_else(|| CaseStatus::Failed("this outcome requires `security_context`".to_owned()))?;
    let security_context =
        read_bytes(&security_context_path(model_folder, context_name)).map_err(failed)?;
    Ok(PreparedCase {
        duckdb,
        handle,
        security_context,
    })
}

/// Prepares the `DuckDB` database of `case` and loads its model, or returns the status that
/// decides the case: a load that fails or a fixture that cannot be read.
fn prepare_database_and_model(
    model_folder: &Path,
    case: &Case,
) -> Result<(Connection, Box<Handle>), CaseStatus> {
    let failed = |error: RunnerError| CaseStatus::Failed(error.to_string());
    let fixture = ModelFixture::read(model_folder).map_err(failed)?;
    let duckdb = prepare_database(&fixture).map_err(|error| {
        failed(RunnerError::new(
            RunnerErrorKind::Database(error),
            model_folder,
        ))
    })?;
    let documents = ModelDocuments::read(model_folder).map_err(failed)?;
    let options = resolve_load_options(case.load_options.as_ref());
    match check_load(case, &documents, &options) {
        LoadOutcome::Checked(status) => Err(status),
        LoadOutcome::Loaded(handle) => Ok((duckdb, handle)),
    }
}

/// A failed status names the sources of the fixture that could not be created on `ClickHouse`.
fn with_unloaded_sources(status: CaseStatus, not_created: &[String]) -> CaseStatus {
    match status {
        CaseStatus::Failed(reason) if !not_created.is_empty() => CaseStatus::Failed(format!(
            "{reason}; not created on ClickHouse: {}",
            not_created.join("; ")
        )),
        status => status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e6_7_10_refuses_a_server_whose_time_zone_is_utc() {
        for name in ["UTC", "Etc/UTC", "GMT", ""] {
            assert!(is_utc(name), "{name}");
        }
        assert!(!is_utc("America/La_Paz"));
    }
}
