//! The wrapper calls of the conformance cases: every command a case's inputs can run, each
//! distinct call once (ARC-6.9).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use menura_core::{Argument, CommandName};
use serde_json::{Map, Value, json};

use super::call::{CallValue, Document, WrapperCall};
use crate::case::{Case, CaseLoadOptions, Outcome};
use crate::case_file::read_case;
use crate::error::RunnerError;
use crate::fixture::{read_bytes, security_context_path};
use crate::runner::{area_folders, case_files};

/// The model folder whose load options are probed for usage errors.
const PROBE_MODEL: &str = "slice";

/// The calls of every case of `conformance_directory/cases/`, in case order, each distinct call
/// once.
///
/// A case gives `validate`, `migrate`, `dependencies` and `integrity-checks` of its model;
/// `describe` when it names a security context; `compile` and `explain` when it has a query
/// and expects `error`, `plan` or `runtime_error`; and `diff` from the model of the case before
/// it to its own. Load and compile options are those of the case (CS-2.7, CS-2.18), with
/// `audit: true` when it expects an audit record (CS-2.13).
///
/// # Errors
///
/// Returns a [`RunnerError`] when the layout or a case file cannot be read.
pub fn calls_of_cases(conformance_directory: &Path) -> Result<Vec<WrapperCall>, RunnerError> {
    let mut builder = CallBuilder::default();
    let mut previous: Option<ModelInputs> = None;
    for area_folder in area_folders(&conformance_directory.join("cases"), None)? {
        for case_path in case_files(&area_folder)? {
            let case = read_case(&case_path)?;
            let inputs = ModelInputs::read(conformance_directory, &case)?;
            builder.add_case(&case, &inputs, previous.as_ref())?;
            previous = Some(inputs);
        }
    }
    Ok(builder.calls)
}

/// Calls of `validate` on the model `slice` whose load options the CLI refuses: an unknown
/// option, `null`, a value of the wrong type, and a value that is not an object (ARC-5.5,
/// ARC-6.4, ARC-6.7).
///
/// # Errors
///
/// Returns a [`RunnerError`] when the model files cannot be read.
pub fn usage_probes(conformance_directory: &Path) -> Result<Vec<WrapperCall>, RunnerError> {
    let folder = conformance_directory.join("models").join(PROBE_MODEL);
    let model = fixture(folder.join("model.yaml"))?;
    let catalog_schema = fixture(folder.join("catalog-schema.json"))?;
    let probes = [
        json!({"colour": 1}),
        json!({"namespace": null}),
        json!({"namespace": 3}),
        json!([]),
    ];
    Ok(probes
        .into_iter()
        .enumerate()
        .map(|(index, options)| WrapperCall {
            case: format!("usage-probe-{}", index + 1),
            command: CommandName::Validate,
            arguments: vec![
                (Argument::Model, model.clone()),
                (Argument::CatalogSchema, catalog_schema.clone()),
                (Argument::LoadOptions, CallValue::Options(options)),
            ],
        })
        .collect())
}

/// The model inputs of a case: its fixture documents and load options.
struct ModelInputs {
    folder: PathBuf,
    model: CallValue,
    catalog_schema: CallValue,
    catalog_state: Option<CallValue>,
    load_options: Option<CallValue>,
}

impl ModelInputs {
    fn read(conformance_directory: &Path, case: &Case) -> Result<Self, RunnerError> {
        let folder = conformance_directory.join("models").join(&case.model);
        let state_path = folder.join("catalog-state.json");
        Ok(Self {
            model: fixture(folder.join("model.yaml"))?,
            catalog_schema: fixture(folder.join("catalog-schema.json"))?,
            catalog_state: state_path
                .is_file()
                .then(|| fixture(state_path))
                .transpose()?,
            load_options: case
                .load_options
                .as_ref()
                .map(|options| CallValue::Options(load_options_value(options))),
            folder,
        })
    }

    /// `model` and `catalog_schema`.
    fn model_arguments(&self) -> Vec<(Argument, CallValue)> {
        vec![
            (Argument::Model, self.model.clone()),
            (Argument::CatalogSchema, self.catalog_schema.clone()),
        ]
    }
}

/// Collects distinct calls in order.
#[derive(Default)]
struct CallBuilder {
    calls: Vec<WrapperCall>,
    seen: BTreeSet<String>,
}

impl CallBuilder {
    fn add_case(
        &mut self,
        case: &Case,
        inputs: &ModelInputs,
        previous: Option<&ModelInputs>,
    ) -> Result<(), RunnerError> {
        let model = inputs.model_arguments();
        self.add(case, CommandName::Validate, model.clone(), inputs);
        let migrate = vec![(Argument::Model, inputs.model.clone())];
        self.add(case, CommandName::Migrate, migrate, inputs);
        self.add(case, CommandName::Dependencies, model.clone(), inputs);
        self.add(case, CommandName::IntegrityChecks, model.clone(), inputs);
        if let Some(previous) = previous {
            self.add(
                case,
                CommandName::Diff,
                diff_arguments(previous, inputs),
                inputs,
            );
        }
        let Some(context_file) = &case.security_context else {
            return Ok(());
        };
        let mut handle = model;
        if let Some(state) = &inputs.catalog_state {
            handle.push((Argument::CatalogState, state.clone()));
        }
        let context = fixture(security_context_path(&inputs.folder, context_file))?;
        let mut describe = handle.clone();
        describe.push((Argument::SecurityContext, context.clone()));
        self.add(case, CommandName::Describe, describe, inputs);
        let compiles = matches!(
            case.expect.outcome,
            Outcome::Error | Outcome::Plan | Outcome::RuntimeError
        );
        if let (true, Some(query)) = (compiles, &case.query) {
            let mut compile = handle;
            compile.push((Argument::Query, generated(query.to_string().into_bytes())));
            compile.push((Argument::SecurityContext, context));
            compile.push((Argument::CompileOptions, compile_options_value(case)));
            self.add(case, CommandName::Compile, compile.clone(), inputs);
            self.add(case, CommandName::Explain, compile, inputs);
        }
        Ok(())
    }

    /// Adds the call unless an equal one was added before; the case's load options are
    /// appended.
    fn add(
        &mut self,
        case: &Case,
        command: CommandName,
        mut arguments: Vec<(Argument, CallValue)>,
        inputs: &ModelInputs,
    ) {
        if let Some(options) = &inputs.load_options {
            arguments.push((Argument::LoadOptions, options.clone()));
        }
        let call = WrapperCall {
            case: case.id.to_string(),
            command,
            arguments,
        };
        if self.seen.insert(call_key(&call)) {
            self.calls.push(call);
        }
    }
}

/// The arguments of `diff` from the model of `old` to that of `new` (ARC-5.6).
fn diff_arguments(old: &ModelInputs, new: &ModelInputs) -> Vec<(Argument, CallValue)> {
    vec![
        (Argument::OldModel, old.model.clone()),
        (Argument::OldCatalogSchema, old.catalog_schema.clone()),
        (Argument::NewModel, new.model.clone()),
        (Argument::NewCatalogSchema, new.catalog_schema.clone()),
    ]
}

/// A key equal for equal calls: the command and, for each argument, its file or value.
fn call_key(call: &WrapperCall) -> String {
    let mut key = call.command.name().to_owned();
    for (argument, value) in &call.arguments {
        let value = match value {
            CallValue::Document(Document {
                path: Some(path), ..
            }) => path.display().to_string(),
            CallValue::Document(document) => String::from_utf8_lossy(&document.bytes).into_owned(),
            CallValue::Options(options) => options.to_string(),
        };
        key.push('\u{0}');
        key.push_str(argument.name());
        key.push('=');
        key.push_str(&value);
    }
    key
}

/// The load options a case sets, as the JSON object of section 3.5 (CS-2.7).
fn load_options_value(options: &CaseLoadOptions) -> Value {
    let mut object = Map::new();
    if let Some(format) = options.ossie_format {
        object.insert("ossie_format".to_owned(), json!(format));
    }
    if let Some(name) = &options.model_name {
        object.insert("model_name".to_owned(), json!(name));
    }
    if let Some(namespace) = &options.namespace {
        object.insert("namespace".to_owned(), json!(namespace));
    }
    if let Some(grant) = options.grant_authenticated_to_all {
        object.insert("grant_authenticated_to_all".to_owned(), json!(grant));
    }
    Value::Object(object)
}

/// The compile options of a case: its `actor` (CS-2.18), and `audit: true` when it expects an
/// audit record (CS-2.13).
fn compile_options_value(case: &Case) -> CallValue {
    let mut object = Map::new();
    if case.expect.audit.is_some() {
        object.insert("audit".to_owned(), Value::Bool(true));
    }
    let actor = case
        .compile_options
        .as_ref()
        .and_then(|options| options.actor.as_ref());
    if let Some(actor) = actor {
        object.insert("actor".to_owned(), json!(actor));
    }
    CallValue::Options(Value::Object(object))
}

/// A document read from the fixture file at `path`.
fn fixture(path: PathBuf) -> Result<CallValue, RunnerError> {
    let bytes = read_bytes(&path)?;
    Ok(CallValue::Document(Document {
        path: Some(path),
        bytes,
    }))
}

/// A document the runner writes itself.
fn generated(bytes: Vec<u8>) -> CallValue {
    CallValue::Document(Document { path: None, bytes })
}

#[cfg(test)]
mod tests {
    use menura_core::{LoadOptions, parse_compile_options, parse_load_options};

    use super::*;
    use crate::compile_options::compile_options;
    use crate::load_options::resolve_load_options;

    fn conformance_directory() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
    }

    /// The calls of `case` alone, without a `diff` from a case before it.
    fn calls_of(case: &Case) -> Vec<WrapperCall> {
        let inputs = ModelInputs::read(&conformance_directory(), case).unwrap();
        let mut builder = CallBuilder::default();
        builder.add_case(case, &inputs, None).unwrap();
        builder.calls
    }

    /// The load options of `call`, read as the CLI reads its file (ARC-5.5).
    fn load_options_of(call: &WrapperCall) -> LoadOptions {
        call.options(Argument::LoadOptions)
            .map_or_else(LoadOptions::default, |options| {
                parse_load_options(options.to_string().as_bytes()).unwrap()
            })
    }

    /// PLT-1.2, ARC-6.9: every call of a case passes the documents and options the runner
    /// compiles that case with, so the wrappers and the runner check the same inputs.
    #[test]
    fn arc_6_9_the_wrapper_calls_give_the_inputs_the_runner_compiles_each_case_with() {
        let directory = conformance_directory();
        let mut compiled = 0;
        for area_folder in area_folders(&directory.join("cases"), None).unwrap() {
            for case_path in case_files(&area_folder).unwrap() {
                let case = read_case(&case_path).unwrap();
                let folder = directory.join("models").join(&case.model);
                let model = read_bytes(&folder.join("model.yaml")).unwrap();
                let schema = read_bytes(&folder.join("catalog-schema.json")).unwrap();
                for call in calls_of(&case) {
                    assert_eq!(call.case, case.id.to_string());
                    assert_eq!(
                        load_options_of(&call),
                        resolve_load_options(case.load_options.as_ref()),
                        "{}",
                        call.case
                    );
                    if call.document(Argument::Model).is_some() {
                        assert_eq!(call.document(Argument::Model), Some(model.as_slice()));
                    }
                    if call.document(Argument::CatalogSchema).is_some() {
                        assert_eq!(
                            call.document(Argument::CatalogSchema),
                            Some(schema.as_slice())
                        );
                    }
                    if call.command != CommandName::Compile {
                        continue;
                    }
                    compiled += 1;
                    let query = serde_json::to_vec(case.query.as_ref().unwrap()).unwrap();
                    assert_eq!(call.document(Argument::Query), Some(query.as_slice()));
                    let context_file = case.security_context.as_deref().unwrap();
                    let context =
                        read_bytes(&security_context_path(&folder, context_file)).unwrap();
                    assert_eq!(
                        call.document(Argument::SecurityContext),
                        Some(context.as_slice())
                    );
                    let options = call.options(Argument::CompileOptions).unwrap();
                    assert_eq!(
                        parse_compile_options(options.to_string().as_bytes()).unwrap(),
                        compile_options(&case),
                        "{}",
                        call.case
                    );
                }
            }
        }
        assert!(compiled > 0);
    }
}
