//! Prepares one Apache Ossie case as CS-4.4 and CS-4.6 say: the fixture loaded into a fresh
//! database, the catalog read from the created tables, the query wrapped in an envelope and the
//! expected result.

use std::path::Path;

use duckdb::Connection;
use menura_contracts::types::audit_record::{LoadOptions, OssieFormat};
use serde_json::{Value, json};

use crate::database::prepare_database;
use crate::error::{RunnerError, RunnerErrorKind};
use crate::fixture::{ModelFixture, read_text};
use crate::load_options::DEFAULT_NAMESPACE;
use crate::ossie::created_tables::{CreatedRelation, read_created_tables};
use crate::ossie::expected::{
    ExpectedResult, WideTableFixture, expected_rows_of, parse_wide_table_expectation,
};
use crate::ossie::suite::{OssieCase, Suite};

/// Everything the compiler needs to run a case, and what the case expects.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparedCase {
    /// The Apache Ossie model document, as read from its YAML file.
    pub model: Value,
    /// The text of the model file, which the compiler loads unchanged (CS-3.1).
    pub model_text: String,
    /// `ossie_format: development` and `grant_authenticated_to_all: true` (CS-4.4).
    pub load_options: LoadOptions,
    /// The tables the fixture created, from which the catalog schema is built (CS-4.4).
    pub created_tables: Vec<CreatedRelation>,
    /// The query envelope (CS-4.4, CS-4.6).
    pub query: Value,
    /// The runner security context (CS-4.5).
    pub security_context: Value,
    /// The expected rows or first error code.
    pub expected: ExpectedResult,
}

/// The fixture files of a case, relative to its suite folder.
struct FixtureFiles {
    data_script: &'static str,
    model: &'static str,
}

/// Prepares `case` in a fresh database, with the runner security context `security_context`,
/// and returns the database with the fixture loaded, on which the case runs.
///
/// # Errors
///
/// Returns a [`RunnerError`] when a case or fixture file cannot be read or parsed, or when the
/// database cannot be prepared or `expected.sql` fails.
pub fn prepare_case(
    case: &OssieCase,
    security_context: &Value,
) -> Result<(PreparedCase, Connection), RunnerError> {
    let folder = case.folder();
    let (files, query, expected) = case_inputs(case, &folder)?;
    let model_text = read_text(&case.suite_directory.join(files.model))?;
    let model = parse_model(&model_text, &case.suite_directory.join(files.model))?;
    let fixture = ModelFixture {
        sources: dataset_sources(&model),
        data_script: read_text(&case.suite_directory.join(files.data_script))?,
    };
    let database_error = |error| RunnerError::new(RunnerErrorKind::Database(error), &folder);
    let connection = prepare_database(&fixture).map_err(database_error)?;
    let created_tables = read_created_tables(&connection).map_err(database_error)?;
    let expected = if let Some(expected) = expected {
        expected
    } else {
        let sql = read_text(&folder.join("expected.sql"))?;
        ExpectedResult::Rows(expected_rows_of(&connection, &sql).map_err(database_error)?)
    };
    let prepared = PreparedCase {
        model,
        model_text,
        load_options: load_options(),
        created_tables,
        query,
        security_context: security_context.clone(),
        expected,
    };
    Ok((prepared, connection))
}

/// The fixture files, the query envelope and, for a wide-table case, the expected result read
/// from the case folder. A Layer 2 case gets its expected result from `expected.sql` once the
/// fixture is loaded (CS-4.6).
fn case_inputs(
    case: &OssieCase,
    folder: &Path,
) -> Result<(FixtureFiles, Value, Option<ExpectedResult>), RunnerError> {
    match case.suite {
        Suite::WideTable => {
            let path = folder.join("expected.json");
            let expectation = parse_wide_table_expectation(&read_text(&path)?)
                .map_err(|kind| RunnerError::new(kind, &path))?;
            let question = read_json(&folder.join("query.json"))?;
            Ok((
                wide_table_files(expectation.fixture),
                query_envelope("query", question),
                Some(expectation.result),
            ))
        }
        Suite::Query => {
            let text = read_text(&folder.join("query.ossie.sql"))?;
            Ok((
                SCOTT_FILES,
                query_envelope("sql", Value::String(text)),
                None,
            ))
        }
    }
}

const SCOTT_FILES: FixtureFiles = FixtureFiles {
    data_script: "scott/scott.sql",
    model: "scott/scott_model.ossie.yaml",
};

/// The files of a wide-table fixture (upstream `compliance/wide_table/README.md`, "Fixtures").
fn wide_table_files(fixture: WideTableFixture) -> FixtureFiles {
    match fixture {
        WideTableFixture::Retail => FixtureFiles {
            data_script: "retail/retail.sql",
            model: "retail/retail_model.ossie.yaml",
        },
        WideTableFixture::Stores => FixtureFiles {
            data_script: "stores/stores.sql",
            model: "stores/stores_model.ossie.yaml",
        },
        WideTableFixture::StoresAmbiguous => FixtureFiles {
            data_script: "stores/stores.sql",
            model: "stores/stores_ambiguous_model.ossie.yaml",
        },
    }
}

/// The load options of CS-4.4.
fn load_options() -> LoadOptions {
    LoadOptions {
        grant_authenticated_to_all: true,
        ossie_format: OssieFormat::Development,
        namespace: DEFAULT_NAMESPACE.to_owned(),
        model_name: None,
    }
}

/// A query envelope with `time_zone: UTC` around the question, under `form` (`query` for the
/// wide-table form, `sql` for the SQL form). The question is kept unchanged, so a query the
/// compiler must reject reaches it as the suite wrote it.
fn query_envelope(form: &str, question: Value) -> Value {
    let mut envelope = json!({
        "contract": "menura.query",
        "contract_version": "1.0.0",
        "time_zone": "UTC",
    });
    if let Value::Object(members) = &mut envelope {
        members.insert(form.to_owned(), question);
    }
    envelope
}

/// The `source` of every dataset of the model, in document order.
fn dataset_sources(model: &Value) -> Vec<String> {
    model
        .get("datasets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|dataset| dataset.get("source").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

fn parse_model(text: &str, path: &Path) -> Result<Value, RunnerError> {
    serde_saphyr::from_str(text)
        .map_err(|error| RunnerError::new(RunnerErrorKind::Malformed(error.to_string()), path))
}

fn read_json(path: &Path) -> Result<Value, RunnerError> {
    serde_json::from_str(&read_text(path)?)
        .map_err(|error| RunnerError::new(RunnerErrorKind::Malformed(error.to_string()), path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs_4_4_wraps_the_query_in_an_envelope_in_utc() {
        let question = json!({"dimensions": ["customers.region"], "unknown": null});
        assert_eq!(
            query_envelope("query", question.clone()),
            json!({
                "contract": "menura.query",
                "contract_version": "1.0.0",
                "time_zone": "UTC",
                "query": question,
            })
        );
    }

    #[test]
    fn cs_4_6_wraps_the_sql_text_in_the_sql_field() {
        let envelope = query_envelope("sql", Value::String("SELECT 1".to_owned()));
        assert_eq!(envelope["sql"], "SELECT 1");
        assert_eq!(envelope["time_zone"], "UTC");
    }

    #[test]
    fn cs_4_4_loads_in_the_development_format_granting_authenticated_to_all() {
        let options = load_options();
        assert_eq!(options.ossie_format, OssieFormat::Development);
        assert!(options.grant_authenticated_to_all);
    }

    #[test]
    fn cs_4_2_reads_the_sources_of_the_datasets() {
        let model = json!({"datasets": [{"source": "retail.orders"}, {"source": "a.b.c"}]});
        assert_eq!(dataset_sources(&model), ["retail.orders", "a.b.c"]);
    }
}
