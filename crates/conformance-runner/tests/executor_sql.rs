//! What the SQL of every compiled plan of the suite leaves to the executor (EXE-1.2, EXE-1.4):
//! every golden SQL file is one read-only statement, `SELECT` or `WITH`, so it never installs
//! or loads an extension and never attaches, detaches or switches a database.

#![expect(
    clippy::disallowed_methods,
    reason = "A2: the test reads the golden SQL files of the conformance directory"
)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Every `.sql` file under the case folders of the suite, with its text.
fn golden_sql_files() -> io::Result<Vec<(PathBuf, String)>> {
    let cases = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/cases");
    let mut files = Vec::new();
    for area in fs::read_dir(&cases)? {
        let area = area?.path();
        if !area.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&area)? {
            let path = entry?.path();
            if path.extension().is_some_and(|extension| extension == "sql") {
                let sql = fs::read_to_string(&path)?;
                files.push((path, sql));
            }
        }
    }
    files.sort();
    Ok(files)
}

/// Why `sql` is not a single `SELECT` or `WITH` statement, or `None`.
fn statement_mismatch(sql: &str) -> Option<&'static str> {
    let start = sql.trim_start().to_ascii_uppercase();
    if !(start.starts_with("SELECT") || start.starts_with("WITH")) {
        return Some("it does not start with SELECT or WITH");
    }
    sql.contains(';')
        .then_some("it holds a `;`, so it may hold another statement")
}

/// Why some golden SQL files are not single read-only statements, one line each; empty when
/// every file is one.
fn read_only_mismatches() -> io::Result<Vec<String>> {
    let files = golden_sql_files()?;
    if files.is_empty() {
        return Ok(vec!["no golden SQL file found".to_owned()]);
    }
    Ok(files
        .iter()
        .filter_map(|(path, sql)| {
            statement_mismatch(sql).map(|reason| format!("{}: {reason}", path.display()))
        })
        .collect())
}

#[test]
fn exe_1_2_no_plan_installs_or_loads_an_extension() {
    let mismatches = read_only_mismatches().unwrap();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert!(statement_mismatch("INSTALL icu").is_some());
    assert!(statement_mismatch("LOAD icu").is_some());
    assert!(statement_mismatch("SELECT 1; LOAD icu").is_some());
}

#[test]
fn exe_1_4_no_plan_attaches_detaches_or_switches_a_database() {
    let mismatches = read_only_mismatches().unwrap();
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
    assert!(statement_mismatch("ATTACH ':memory:' AS shop").is_some());
    assert!(statement_mismatch("DETACH shop").is_some());
    assert!(statement_mismatch("USE shop").is_some());
    assert!(statement_mismatch("WITH a AS (SELECT 1) SELECT * FROM a").is_none());
}
