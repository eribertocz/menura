"""ARC-6.9, A4: the Python package gives, for every call of the conformance cases, the documents
`core` gives, byte for byte.

Usage: check_wrapper_parity.py <manifest>

The manifest is written by `wrapper-parity` (crates/conformance-runner), which also checks the
CLI and the MCP server: each call names its command, its arguments (a fixture `path`, a
document `text` or an `options` value) and what `core` gives (`document`, `is_error`,
`audit_record`, or a `usage_error` message). Each difference is printed with its case, the
wrapper `python` and the first differing line; any difference exits 1.
"""

import json
import pathlib
import sys

EXCERPT_RADIUS = 40


def first_difference(expected, actual):
    """The first line where `actual` differs from `expected`, as text, or None when equal."""
    if expected == actual:
        return None
    expected_lines, actual_lines = expected.split("\n"), actual.split("\n")
    line = 0
    while (
        line < len(expected_lines)
        and line < len(actual_lines)
        and expected_lines[line] == actual_lines[line]
    ):
        line += 1
    left = expected_lines[line] if line < len(expected_lines) else None
    right = actual_lines[line] if line < len(actual_lines) else None
    column = 0
    while left and right and column < min(len(left), len(right)) and left[column] == right[column]:
        column += 1

    def side(text):
        if text is None:
            return "no line"
        start = max(column - EXCERPT_RADIUS, 0)
        return "`" + text[start : column + EXCERPT_RADIUS] + "`"

    return f"line {line + 1}, column {column + 1}: expected {side(left)}, found {side(right)}"


def argument_value(value):
    """The Python value of a manifest argument: bytes for a document, the options otherwise."""
    if "path" in value:
        return pathlib.Path(value["path"]).read_bytes()
    if "text" in value:
        return value["text"].encode("utf-8")
    return value["options"]


class Outcome:
    """What one call gave: a result, a load error, or a usage error."""

    def __init__(self, document=None, is_error=None, audit_record=None, usage_error=None):
        self.document = document
        self.is_error = is_error
        self.audit_record = audit_record
        self.usage_error = usage_error


def run_call(package, command, arguments):
    """Runs `command` through `package` and returns its outcome (ARC-6.4, ARC-6.5)."""
    try:
        result = call_package(package, command, arguments)
    except package.CompileError as error:
        return Outcome(document=error.document, is_error=True)
    except ValueError as error:
        return Outcome(usage_error=str(error))
    return Outcome(result.document, result.is_error, getattr(result, "audit_record", None))


def call_package(package, command, arguments):
    """The `Result` of `command` called with `arguments`, as `menura` names its functions."""
    get = arguments.get
    load_options = get("load_options")
    if command == "validate":
        return package.validate(get("model"), get("catalog_schema"), load_options)
    if command == "migrate":
        return package.migrate(get("model"), load_options)
    if command == "diff":
        documents = ("old_model", "old_catalog_schema", "new_model", "new_catalog_schema")
        return package.diff(*(get(name) for name in documents), load_options)
    state = get("catalog_state") if command in ("describe", "compile", "explain") else None
    handle = package.Handle.load(get("model"), get("catalog_schema"), state, load_options)
    if command == "dependencies":
        return handle.dependencies()
    if command == "integrity_checks":
        return handle.integrity_checks()
    if command == "describe":
        return handle.describe(get("security_context"))
    query, context, options = get("query"), get("security_context"), get("compile_options")
    if command == "compile":
        return handle.compile(query, context, options)
    return handle.explain(query, context, options)


def differences(expected, outcome):
    """What differs between `outcome` and `expected`, as a list of texts."""
    if "usage_error" in expected:
        if outcome.usage_error is None:
            return ["no ValueError was raised"]
        found = first_difference(expected["usage_error"], outcome.usage_error)
        return [f"ValueError: {found}"] if found else []
    if outcome.usage_error is not None:
        return [f"ValueError: {outcome.usage_error}"]
    found = []
    if outcome.is_error != expected["is_error"]:
        found.append(f"is_error {outcome.is_error}, expected {expected['is_error']}")
    document = first_difference(expected["document"], outcome.document)
    if document:
        found.append(f"document: {document}")
    if expected["audit_record"] != outcome.audit_record:
        record = first_difference(expected["audit_record"] or "", outcome.audit_record or "")
        found.append(f"audit record: {record or 'present on one side only'}")
    return found


def with_state_outcome(package, arguments):
    """`describe` through `Handle.with_state`, or None when the model does not load without its
    catalog state (ARC-5.2, ARC-6.5)."""
    try:
        handle = package.Handle.load(
            arguments["model"], arguments["catalog_schema"], None, arguments.get("load_options")
        )
    except package.CompileError:
        return None
    result = handle.with_state(arguments["catalog_state"]).describe(arguments["security_context"])
    return Outcome(result.document, result.is_error)


def check(manifest, package):
    """Every difference between `package` and the expected documents of `manifest`."""
    found = []
    for call in manifest["calls"]:
        arguments = {name: argument_value(value) for name, value in call["arguments"].items()}
        command, expected = call["command"], call["expected"]
        outcomes = [("", run_call(package, command, arguments))]
        if command == "describe" and "catalog_state" in arguments:
            outcomes.append((" with_state", with_state_outcome(package, arguments)))
        for form, outcome in outcomes:
            if outcome is None:
                continue
            for difference in differences(expected, outcome):
                found.append(f"case {call['case']}: python `{command}`{form}: {difference}")
    return found


def main(arguments):
    if len(arguments) != 1:
        print("usage: check_wrapper_parity.py <manifest>", file=sys.stderr)
        return 2
    import menura

    manifest = json.loads(pathlib.Path(arguments[0]).read_text(encoding="utf-8"))
    found = check(manifest, menura)
    for difference in found:
        print(difference)
    print(f"{len(manifest['calls'])} calls through the Python package, {len(found)} differences")
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
