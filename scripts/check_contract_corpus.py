"""Validate the contract corpus with the reference JSON Schema implementation.

Each file of spec/contracts/corpus/ lists cases: a published example, a patch (a subset of
RFC 6902: add, replace, remove) and the expected verdict. This script checks every case with
Python jsonschema (Draft 2020-12, format checker on). The Rust test `corpus` checks the same
cases with the compiler's evaluator, so the two implementations cannot disagree on any case.

Usage: python3 scripts/check_contract_corpus.py
Exit status: 0 when every verdict matches, 1 otherwise.
"""
import copy
import json
import pathlib
import sys

import jsonschema

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONTRACTS = ROOT / "spec" / "contracts"
CORPUS = CONTRACTS / "corpus"


def load(path):
    """Read a JSON file."""
    return json.loads(path.read_text(encoding="utf-8"))


def tokens(pointer):
    """Split a JSON Pointer (RFC 6901) into unescaped reference tokens."""
    if pointer == "":
        return []
    return [token.replace("~1", "/").replace("~0", "~") for token in pointer[1:].split("/")]


def apply(document, patch):
    """Apply add, replace and remove operations to a copy of the document."""
    result = copy.deepcopy(document)
    for operation in patch:
        *parents, last = tokens(operation["path"])
        target = result
        for token in parents:
            target = target[int(token)] if isinstance(target, list) else target[token]
        key = int(last) if isinstance(target, list) else last
        if operation["op"] == "remove":
            del target[key]
        elif operation["op"] == "replace":
            if isinstance(target, dict) and key not in target:
                raise KeyError(f"replace of a missing member {operation['path']}")
            target[key] = operation["value"]
        elif operation["op"] == "add" and isinstance(target, dict):
            target[key] = operation["value"]
        else:
            raise ValueError(f"unsupported operation {operation}")
    return result


def check_file(path):
    """Return one failure message per case whose verdict differs from the expected one."""
    corpus = load(path)
    validator = jsonschema.Draft202012Validator(
        load(CONTRACTS / "schemas" / f"{corpus['schema']}.schema.json"),
        format_checker=jsonschema.FormatChecker(),
    )
    failures = []
    for case in corpus["cases"]:
        example = load(CONTRACTS / "examples" / case.get("example", corpus["example"]))
        valid = validator.is_valid(apply(example, case["patch"]))
        if valid != case["valid"]:
            expected = "valid" if case["valid"] else "invalid"
            failures.append(f"{path.name}: '{case['name']}' should be {expected}")
    return failures


def main():
    """Check every corpus file and print the cases that disagree."""
    if "date-time" not in jsonschema.FormatChecker().checkers:
        print("the date-time format checker needs the rfc3339-validator package")
        sys.exit(1)
    failures = [failure for path in sorted(CORPUS.glob("*.corpus.json")) for failure in check_file(path)]
    for failure in failures:
        print(failure)
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
