//! Regenerating the golden files of explicitly named cases (CS-2.10a): the runner writes the
//! produced output, `sql` as the plan's text unchanged and JSON in stored form (CS-2.10).

use std::fs;
use std::path::{Path, PathBuf};

use menura_core::{CompileOptions, CompileResult, compile};

use crate::canonical_json::JsonValue;
use crate::case::{Case, CaseId, Outcome};
use crate::case_file::read_case;
use crate::error::{RunnerError, RunnerErrorKind};
use crate::fixture::{read_bytes, security_context_path};
use crate::load_options::resolve_load_options;
use crate::load_part::{LoadOutcome, ModelDocuments, check_load};
use crate::runner::list_directory;

/// What regenerating one golden file of a case did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Regeneration {
    /// The golden file was written with the produced output.
    Written(PathBuf),
    /// The golden file was left unchanged, with the reason.
    NotWritten(PathBuf, String),
}

/// Regenerates the golden files of the cases `case_ids`, and of no other case (CS-2.10a).
///
/// For an `outcome: plan` case, `sql` is written as the plan's SQL text and `lineage` in stored
/// form. A golden file whose output `core` cannot produce yet (`audit`, `document`), or a case
/// whose query does not compile, is reported as not written.
///
/// # Errors
///
/// A [`RunnerError`] when a named case does not exist, a case file or a fixture cannot be read,
/// or a golden file cannot be written.
pub fn regenerate(
    conformance_directory: &Path,
    case_ids: &[CaseId],
) -> Result<Vec<Regeneration>, RunnerError> {
    let mut regenerations = Vec::new();
    for case_id in case_ids {
        let case_path = find_case(conformance_directory, case_id)?;
        let case = read_case(&case_path)?;
        let case_folder = case_path.parent().unwrap_or(conformance_directory);
        regenerations.extend(regenerate_case(conformance_directory, case_folder, &case)?);
    }
    Ok(regenerations)
}

/// The case file `cases/<area>/<case_id>.yaml`.
fn find_case(conformance_directory: &Path, case_id: &CaseId) -> Result<PathBuf, RunnerError> {
    let cases_directory = conformance_directory.join("cases");
    let file_name = format!("{case_id}.yaml");
    for area_folder in list_directory(&cases_directory)? {
        let candidate = area_folder.join(&file_name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(RunnerError::new(
        RunnerErrorKind::UnknownCase(case_id.to_string()),
        cases_directory,
    ))
}

fn regenerate_case(
    conformance_directory: &Path,
    case_folder: &Path,
    case: &Case,
) -> Result<Vec<Regeneration>, RunnerError> {
    let expect = &case.expect;
    let mut regenerations: Vec<Regeneration> = [&expect.audit, &expect.document]
        .into_iter()
        .flatten()
        .map(|file_name| {
            Regeneration::NotWritten(
                case_folder.join(file_name),
                "core does not produce this output yet (CS-2.8)".to_owned(),
            )
        })
        .collect();
    let has_plan = matches!(expect.outcome, Outcome::Plan | Outcome::RuntimeError);
    let lineage = expect
        .lineage
        .as_ref()
        .filter(|_| expect.outcome == Outcome::Plan);
    let sql = expect.sql.as_ref().filter(|_| has_plan);
    let golden_files: Vec<&String> = [sql, lineage].into_iter().flatten().collect();
    if golden_files.is_empty() {
        return Ok(regenerations);
    }
    match plan_outputs(conformance_directory, case)? {
        Ok(outputs) => {
            for (file_name, text) in [(sql, outputs.sql), (lineage, outputs.lineage)] {
                if let Some(file_name) = file_name {
                    regenerations.push(write_golden(&case_folder.join(file_name), &text)?);
                }
            }
        }
        Err(reason) => regenerations.extend(golden_files.into_iter().map(|file_name| {
            Regeneration::NotWritten(case_folder.join(file_name), reason.clone())
        })),
    }
    Ok(regenerations)
}

/// The produced text of the golden files of a plan.
struct PlanOutputs {
    /// The plan's SQL text, unchanged.
    sql: String,
    /// The plan's `lineage` in stored form.
    lineage: String,
}

/// Loads the model of `case` and compiles its query; the inner error says why no plan was
/// produced.
fn plan_outputs(
    conformance_directory: &Path,
    case: &Case,
) -> Result<Result<PlanOutputs, String>, RunnerError> {
    let model_folder = conformance_directory.join("models").join(&case.model);
    let documents = ModelDocuments::read(&model_folder)?;
    let options = resolve_load_options(case.load_options.as_ref());
    let LoadOutcome::Loaded(handle) = check_load(case, &documents, &options) else {
        return Ok(Err("the model does not load".to_owned()));
    };
    let (Some(query), Some(context_name)) = (&case.query, &case.security_context) else {
        return Ok(Err(
            "the case has no query or no security context".to_owned()
        ));
    };
    let security_context = read_bytes(&security_context_path(&model_folder, context_name))?;
    let query = serde_json::to_vec(query).map_err(|error| {
        RunnerError::new(RunnerErrorKind::Malformed(error.to_string()), &model_folder)
    })?;
    let options = CompileOptions {
        audit: case.expect.audit.is_some(),
        ..CompileOptions::default()
    };
    let CompileResult::Plan(plan) = compile(&handle, &query, &security_context, &options) else {
        return Ok(Err("the query does not compile".to_owned()));
    };
    Ok(JsonValue::of(&plan.lineage).map(|lineage| PlanOutputs {
        sql: plan.sql.clone(),
        lineage: lineage.stored_form(),
    }))
}

fn write_golden(path: &Path, text: &str) -> Result<Regeneration, RunnerError> {
    fs::write(path, text)
        .map_err(|error| RunnerError::new(RunnerErrorKind::Unwritable(error), path))?;
    Ok(Regeneration::Written(path.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conformance_directory() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
    }

    #[test]
    fn cs_2_10a_an_unknown_case_is_an_error() {
        let error = regenerate(
            &conformance_directory(),
            &[CaseId::new("no-such-case".to_owned())],
        )
        .unwrap_err();
        assert!(matches!(error.kind, RunnerErrorKind::UnknownCase(_)));
    }

    #[test]
    fn cs_2_10a_finds_a_case_in_its_area() {
        let path = find_case(
            &conformance_directory(),
            &CaseId::new("cs-lineage-001".to_owned()),
        )
        .unwrap();
        assert!(
            path.ends_with("cases/cs/cs-lineage-001.yaml"),
            "{}",
            path.display()
        );
    }

    #[test]
    fn cs_2_10a_an_output_core_cannot_produce_yet_is_not_written() {
        let case = crate::case_file::parse_case(
            "id: next-describe-001\nrules: [CS-2.15]\nmodel: slice\n\
             security_context: auditor.json\nexpect: {outcome: describe, document: d.json}\n",
        )
        .unwrap();
        let folder = Path::new("cases/next");
        let regenerations = regenerate_case(&conformance_directory(), folder, &case).unwrap();
        assert_eq!(
            regenerations,
            vec![Regeneration::NotWritten(
                folder.join("d.json"),
                "core does not produce this output yet (CS-2.8)".to_owned()
            )]
        );
    }

    #[test]
    fn cs_2_10a_the_sql_of_a_runtime_error_case_is_regenerated() {
        let original = conformance_directory().join("cases/cs/cs-golden-sql-001.sql");
        let folder = std::env::temp_dir().join("menura-cs-2-10a-runtime-error");
        fs::create_dir_all(&folder).unwrap();
        let text =
            fs::read_to_string(conformance_directory().join("cases/cs/cs-golden-sql-001.yaml"))
                .unwrap();
        let mut case = crate::case_file::parse_case(&text).unwrap();
        case.expect.outcome = Outcome::RuntimeError;
        case.expect.error = Some("RADD-01".to_owned());
        let regenerations = regenerate_case(&conformance_directory(), &folder, &case).unwrap();
        let file_name = case.expect.sql.clone().unwrap();
        assert_eq!(
            regenerations,
            vec![Regeneration::Written(folder.join(&file_name))]
        );
        assert_eq!(
            fs::read_to_string(folder.join(&file_name)).unwrap(),
            fs::read_to_string(original).unwrap()
        );
    }

    #[test]
    fn cs_2_10a_the_produced_outputs_are_in_stored_form() {
        let case =
            read_case(&conformance_directory().join("cases/cs/cs-lineage-001.yaml")).unwrap();
        let outputs = plan_outputs(&conformance_directory(), &case)
            .unwrap()
            .unwrap();
        let lineage = JsonValue::parse(&outputs.lineage).unwrap();
        assert_eq!(lineage.stored_form(), outputs.lineage);
        assert!(outputs.sql.starts_with("SELECT") || outputs.sql.starts_with("WITH"));
    }
}
