//! The shared contract corpus (CON-1, A3): the Rust evaluator must give every case the verdict
//! the corpus expects. `scripts/check_contract_corpus.py` checks the same cases with Python
//! `jsonschema` (Draft 2020-12, format checker on), so the two implementations agree.

use std::collections::BTreeSet;

use semantic_compiler_contracts::{ContractKind, validate_structure};
use serde_json::Value;

/// A corpus file and the contract its cases follow.
struct CorpusFile {
    kind: ContractKind,
    text: &'static str,
    schema: &'static str,
}

macro_rules! corpus_file {
    ($kind:ident, $name:literal) => {
        CorpusFile {
            kind: ContractKind::$kind,
            text: include_str!(concat!("corpus/", $name, ".corpus.json")),
            schema: include_str!(concat!(
                "../../../spec/contracts/schemas/",
                $name,
                ".schema.json"
            )),
        }
    };
}

fn corpus_files() -> [CorpusFile; 9] {
    [
        corpus_file!(CatalogSchema, "catalog-schema"),
        corpus_file!(CatalogState, "catalog-state"),
        corpus_file!(Query, "query"),
        corpus_file!(SecurityContext, "security-context"),
        corpus_file!(CompiledPlan, "compiled-plan"),
        corpus_file!(CompileError, "compile-error"),
        corpus_file!(AuditRecord, "audit-record"),
        corpus_file!(ModelDescription, "model-description"),
        corpus_file!(IntegrityChecks, "integrity-checks"),
    ]
}

/// The text of a published example, by file name.
fn example_text(file: &str) -> Option<&'static str> {
    let text = match file {
        "catalog-schema.example.json" => {
            include_str!("../../../spec/contracts/examples/catalog-schema.example.json")
        }
        "catalog-state.example.json" => {
            include_str!("../../../spec/contracts/examples/catalog-state.example.json")
        }
        "query.json-form.example.json" => {
            include_str!("../../../spec/contracts/examples/query.json-form.example.json")
        }
        "query.sql-form.example.json" => {
            include_str!("../../../spec/contracts/examples/query.sql-form.example.json")
        }
        "security-context.example.json" => {
            include_str!("../../../spec/contracts/examples/security-context.example.json")
        }
        "compiled-plan.example.json" => {
            include_str!("../../../spec/contracts/examples/compiled-plan.example.json")
        }
        "compile-error.example.json" => {
            include_str!("../../../spec/contracts/examples/compile-error.example.json")
        }
        "audit-record.example.json" => {
            include_str!("../../../spec/contracts/examples/audit-record.example.json")
        }
        "model-description.example.json" => {
            include_str!("../../../spec/contracts/examples/model-description.example.json")
        }
        "integrity-checks.example.json" => {
            include_str!("../../../spec/contracts/examples/integrity-checks.example.json")
        }
        _ => return None,
    };
    Some(text)
}

/// Applies one `add`, `replace` or `remove` operation (RFC 6902 subset). `None` when the
/// operation is malformed or its target does not exist.
fn apply_operation(document: &mut Value, operation: &Value) -> Option<()> {
    let path = operation.get("path")?.as_str()?;
    let (parent_path, last) = path.rsplit_once('/')?;
    let name = last.replace("~1", "/").replace("~0", "~");
    let parent = document.pointer_mut(parent_path)?;
    match (operation.get("op")?.as_str()?, parent) {
        ("remove", Value::Object(members)) => members.remove(&name).map(|_| ()),
        ("remove", Value::Array(items)) => {
            let index: usize = name.parse().ok()?;
            (index < items.len()).then(|| {
                items.remove(index);
            })
        }
        ("replace", parent) => {
            *parent.pointer_mut(&format!("/{last}"))? = operation.get("value")?.clone();
            Some(())
        }
        ("add", Value::Object(members)) => {
            members.insert(name, operation.get("value")?.clone());
            Some(())
        }
        _ => None,
    }
}

/// The case's document: its example with its patch applied.
fn case_document(case: &Value, default_example: &str) -> Option<Value> {
    let file = case
        .get("example")
        .and_then(Value::as_str)
        .unwrap_or(default_example);
    let mut document: Value = serde_json::from_str(example_text(file)?).ok()?;
    for operation in case.get("patch")?.as_array()? {
        apply_operation(&mut document, operation)?;
    }
    Some(document)
}

/// Collects the keywords of `schema` and of every subschema below it, except annotations.
fn schema_keywords(schema: &Value, found: &mut BTreeSet<String>) {
    let Value::Object(members) = schema else {
        return;
    };
    for (name, member) in members {
        let subschemas: Vec<&Value> = match name.as_str() {
            "properties" | "$defs" => member
                .as_object()
                .into_iter()
                .flat_map(|map| map.values())
                .collect(),
            "allOf" | "anyOf" | "oneOf" => member.as_array().into_iter().flatten().collect(),
            "items" | "additionalProperties" | "not" | "if" | "then" | "else" => vec![member],
            _ => Vec::new(),
        };
        let annotation =
            ["$schema", "$id", "$defs", "title", "description", "default"].contains(&name.as_str());
        if !annotation {
            found.insert(name.clone());
        }
        for subschema in subschemas {
            schema_keywords(subschema, found);
        }
    }
}

fn parse(text: &str) -> Option<Value> {
    serde_json::from_str(text).ok()
}

#[test]
fn con_1_rust_evaluator_agrees_with_every_corpus_verdict() {
    for file in corpus_files() {
        let corpus = parse(file.text).expect("corpus file is JSON");
        let default_example = corpus["example"].as_str().expect("default example");
        for case in corpus["cases"].as_array().expect("cases") {
            let name = case["name"].as_str().expect("case name");
            let document = case_document(case, default_example).expect("patch applies");
            let valid = validate_structure(file.kind, &document).is_ok();
            assert_eq!(
                Some(valid),
                case["valid"].as_bool(),
                "{}: {name}",
                file.kind
            );
        }
    }
}

#[test]
fn a3_corpus_has_an_invalid_case_for_every_schema_keyword() {
    for file in corpus_files() {
        let mut used = BTreeSet::new();
        schema_keywords(&parse(file.schema).expect("schema is JSON"), &mut used);
        let corpus = parse(file.text).expect("corpus file is JSON");
        let covered: BTreeSet<String> = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .filter(|case| case["valid"] == Value::Bool(false))
            .flat_map(|case| case["keywords"].as_array().cloned().unwrap_or_default())
            .filter_map(|keyword| keyword.as_str().map(str::to_owned))
            .collect();
        let missing: Vec<&String> = used.difference(&covered).collect();
        assert!(
            missing.is_empty(),
            "{}: no invalid case for {missing:?}",
            file.kind
        );
    }
}

#[test]
fn a3_every_corpus_file_names_its_contract_schema() {
    for file in corpus_files() {
        let corpus = parse(file.text).expect("corpus file is JSON");
        let schema = parse(file.schema).expect("schema is JSON");
        let stem = corpus["schema"].as_str().expect("schema name");
        let id = schema["$id"].as_str().expect("schema id");
        assert!(
            id.contains(&format!("/{stem}/")),
            "{}: {stem} does not match {id}",
            file.kind
        );
    }
}
