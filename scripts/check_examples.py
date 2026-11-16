"""Validate every contract example against its schema, and the example set against itself.

Usage: python3 scripts/check_examples.py
"""
import json
import pathlib
import sys

import jsonschema
import yaml

CONTRACTS = pathlib.Path(__file__).resolve().parent.parent / "spec" / "contracts"
PAIRS = {
    "audit-record": "audit-record",
    "catalog-schema": "catalog-schema",
    "catalog-state": "catalog-state",
    "compile-error": "compile-error",
    "compiled-plan": "compiled-plan",
    "integrity-checks": "integrity-checks",
    "model-description": "model-description",
    "query.json-form": "query",
    "query.sql-form": "query",
    "security-context": "security-context",
}


def load(path):
    return json.loads(path.read_text(encoding="utf-8"))


def main():
    failures = []
    for example, schema in PAIRS.items():
        validator = jsonschema.Draft202012Validator(
            load(CONTRACTS / "schemas" / f"{schema}.schema.json"),
            format_checker=jsonschema.FormatChecker(),
        )
        for error in validator.iter_errors(load(CONTRACTS / "examples" / f"{example}.example.json")):
            failures.append(f"{example}: {error.message}")

    catalog = load(CONTRACTS / "examples" / "catalog-schema.example.json")
    state = load(CONTRACTS / "examples" / "catalog-state.example.json")
    sources = {relation["source"] for relation in catalog["relations"]}
    if state["schema_document_id"] != catalog["document_id"]:
        failures.append("catalog-state: schema_document_id does not match the schema example")
    for relation in state["relations"]:
        if relation["source"] not in sources:
            failures.append(f"catalog-state: unknown source {relation['source']}")
    model = yaml.safe_load((CONTRACTS / "examples" / "model.example.yaml").read_text(encoding="utf-8"))
    for dataset in model["semantic_model"][0]["datasets"]:
        if dataset["source"] not in sources:
            failures.append(f"model: source {dataset['source']} is not in the catalog schema example")

    for failure in failures:
        print(failure)
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
