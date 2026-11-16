//! Shared fixtures: the published examples of `spec/contracts/examples/`.

use semantic_compiler_contracts::{ContractKind, validate_structure};
use serde_json::Value;

/// One published example and the contract it follows.
pub struct Example {
    pub kind: ContractKind,
    pub file: &'static str,
    pub text: &'static str,
}

macro_rules! example {
    ($kind:ident, $file:literal) => {
        Example {
            kind: ContractKind::$kind,
            file: $file,
            text: include_str!(concat!("../../../../spec/contracts/examples/", $file)),
        }
    };
}

/// Every JSON example of the contracts directory.
pub fn examples() -> Vec<Example> {
    vec![
        example!(CatalogSchema, "catalog-schema.example.json"),
        example!(CatalogState, "catalog-state.example.json"),
        example!(Query, "query.json-form.example.json"),
        example!(Query, "query.sql-form.example.json"),
        example!(SecurityContext, "security-context.example.json"),
        example!(CompiledPlan, "compiled-plan.example.json"),
        example!(CompileError, "compile-error.example.json"),
        example!(AuditRecord, "audit-record.example.json"),
        example!(ModelDescription, "model-description.example.json"),
        example!(IntegrityChecks, "integrity-checks.example.json"),
    ]
}

/// The parsed example of a file.
pub fn example(file: &str) -> Value {
    let found = examples()
        .into_iter()
        .find(|example| example.file == file)
        .expect("known example");
    serde_json::from_str(found.text).expect("examples are JSON")
}

/// Replaces the value at `pointer` (which must exist) with `value`.
pub fn with(mut document: Value, pointer: &str, value: Value) -> Value {
    *document.pointer_mut(pointer).expect("pointer exists") = value;
    document
}

/// Removes the member at `pointer` (which must exist).
pub fn without(mut document: Value, pointer: &str) -> Value {
    let (parent, name) = pointer.rsplit_once('/').expect("pointer has a parent");
    document
        .pointer_mut(parent)
        .and_then(Value::as_object_mut)
        .expect("parent is an object")
        .remove(name)
        .expect("member exists");
    document
}

/// Adds the member `name` to the object at `pointer`.
pub fn adding(mut document: Value, pointer: &str, name: &str, value: Value) -> Value {
    document
        .pointer_mut(pointer)
        .and_then(Value::as_object_mut)
        .expect("target is an object")
        .insert(name.to_owned(), value);
    document
}

/// The pointer of the first schema violation, or `None` when the document is valid.
pub fn violation(kind: ContractKind, document: &Value) -> Option<String> {
    validate_structure(kind, document)
        .err()
        .map(|violation| violation.pointer)
}

pub fn assert_valid(kind: ContractKind, document: &Value) {
    assert_eq!(violation(kind, document), None, "{kind} should be valid");
}

pub fn assert_invalid_at(kind: ContractKind, document: &Value, pointer: &str) {
    assert_eq!(
        violation(kind, document).as_deref(),
        Some(pointer),
        "{kind}"
    );
}
