"""Agent smoke test (spec/09-continuous-integration.md CI-1 `agent-smoke`, rule ARC-6.7).

A fixed client script, with no language model, that drives the built `menura-mcp` binary over
standard input and output on the conformance model `slice`, as the security context `viewer`:

  1. `initialize`, then `tools/list`: all eight tools are listed.
  2. `describe`: a model description.
  3. `compile` of a valid query: a compiled plan.
  4. `compile` with a misspelled metric: `isError: true`, `E_NAME_NOT_FOUND` with
     `alternatives` (PLN-5.4).
  5. `compile` of a metric the context may not use: the same error as a missing metric of the
     same shape, the reference aside (PLN-5.4a).

Exits 0 when every step meets its expectation, and 1 with one line per failed step otherwise.

Usage: python3 -I scripts/ci/agent_smoke.py [path/to/menura-mcp]
Without an argument, runs `target/release/menura-mcp`.
"""
import json
import pathlib
import subprocess
import sys

REPOSITORY = pathlib.Path(__file__).resolve().parents[2]
DEFAULT_BINARY = REPOSITORY / "target" / "release" / "menura-mcp"
MODEL_DIRECTORY = REPOSITORY / "conformance" / "models" / "slice"
SECURITY_CONTEXT = "viewer.json"
TOOLS = ["compile", "dependencies", "describe", "diff", "explain", "integrity_checks", "migrate", "validate"]

VALID_METRIC = "total_amount"
MISSPELLED_METRIC = "total_amont"
MISSPELLED_ALTERNATIVES = ["metric:TOTAL_AMOUNT"]
# `total_target` reads the dataset `targets`, which only the role `manager` may see.
HIDDEN_METRIC = "total_target"
# A name no model object has, of the same length as the hidden one.
MISSING_METRIC = "total_targex"

INITIALIZE = 1
TOOLS_LIST = 2
DESCRIBE = 3
COMPILE_VALID = 4
COMPILE_MISSPELLED = 5
COMPILE_HIDDEN = 6
COMPILE_MISSING = 7
TIMEOUT_SECONDS = 120


def read_documents(directory):
    """The model, catalog schema and security context texts the tools receive."""
    return {
        "model": (directory / "model.yaml").read_text(encoding="utf-8"),
        "catalog_schema": (directory / "catalog-schema.json").read_text(encoding="utf-8"),
        "security_context": (directory / "contexts" / SECURITY_CONTEXT).read_text(encoding="utf-8"),
    }


def query_text(metric):
    """A query document asking for one metric by `sales.region`."""
    query = {
        "contract": "menura.query",
        "contract_version": "1.1.0",
        "time_zone": "UTC",
        "query": {"dimensions": ["sales.region"], "metrics": [{"metric": metric, "alias": "value"}]},
    }
    return json.dumps(query)


def tool_call(identifier, name, arguments):
    """A `tools/call` request."""
    return {"jsonrpc": "2.0", "id": identifier, "method": "tools/call", "params": {"name": name, "arguments": arguments}}


def compile_call(identifier, documents, metric):
    """A `compile` call of the query asking for one metric."""
    return tool_call(identifier, "compile", dict(documents, query=query_text(metric)))


def requests(documents):
    """The fixed message sequence the client sends, in order."""
    return [
        {
            "jsonrpc": "2.0",
            "id": INITIALIZE,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "agent-smoke", "version": "1"},
            },
        },
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": TOOLS_LIST, "method": "tools/list"},
        tool_call(DESCRIBE, "describe", dict(documents)),
        compile_call(COMPILE_VALID, documents, VALID_METRIC),
        compile_call(COMPILE_MISSPELLED, documents, MISSPELLED_METRIC),
        compile_call(COMPILE_HIDDEN, documents, HIDDEN_METRIC),
        compile_call(COMPILE_MISSING, documents, MISSING_METRIC),
    ]


def responses_by_identifier(output):
    """The responses the server wrote, one JSON message per line, keyed by `id`."""
    responses = {}
    for line in output.splitlines():
        if not line.strip():
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(message, dict) and "id" in message:
            responses[message["id"]] = message
    return responses


def result_of(responses, identifier):
    """The `result` object of a response, or a failure reason."""
    response = responses.get(identifier)
    if response is None:
        return None, "no response"
    if "error" in response:
        return None, f"JSON-RPC error {json.dumps(response['error'], sort_keys=True)}"
    result = response.get("result")
    if not isinstance(result, dict):
        return None, "no result object"
    return result, None


def tool_document(responses, identifier, expect_error):
    """The JSON document of a tool result's first text item, or a failure reason."""
    result, reason = result_of(responses, identifier)
    if reason:
        return None, reason
    if bool(result.get("isError")) != expect_error:
        return None, f"isError is {bool(result.get('isError'))}, expected {expect_error}"
    content = result.get("content")
    if not isinstance(content, list) or not content or not isinstance(content[0], dict):
        return None, "no content item"
    try:
        return json.loads(content[0].get("text", "")), None
    except (json.JSONDecodeError, TypeError):
        return None, "the first content item is not a JSON document"


def check_initialize(responses):
    """Step 1: `initialize` answers the protocol revision and `tools/list` lists the eight tools."""
    result, reason = result_of(responses, INITIALIZE)
    if reason:
        return f"initialize: {reason}"
    if result.get("protocolVersion") != "2025-06-18":
        return f"initialize: protocolVersion is {result.get('protocolVersion')!r}, expected '2025-06-18'"
    result, reason = result_of(responses, TOOLS_LIST)
    if reason:
        return f"tools/list: {reason}"
    tools = result.get("tools")
    names = sorted(tool.get("name") for tool in tools if isinstance(tool, dict)) if isinstance(tools, list) else []
    if names != TOOLS:
        return f"tools/list: tools are {names}, expected {TOOLS}"
    return None


def check_describe(responses):
    """Step 2: `describe` returns a model description."""
    document, reason = tool_document(responses, DESCRIBE, expect_error=False)
    if reason:
        return f"describe: {reason}"
    if document.get("contract") != "menura.model_description":
        return f"describe: contract is {document.get('contract')!r}, expected 'menura.model_description'"
    return None


def check_valid_compile(responses):
    """Step 3: `compile` of a valid query returns a compiled plan."""
    document, reason = tool_document(responses, COMPILE_VALID, expect_error=False)
    if reason:
        return f"compile (valid query): {reason}"
    if document.get("contract") != "menura.compiled_plan":
        return f"compile (valid query): contract is {document.get('contract')!r}, expected 'menura.compiled_plan'"
    return None


def first_error(responses, identifier):
    """The first diagnostic of a compile error, or a failure reason."""
    document, reason = tool_document(responses, identifier, expect_error=True)
    if reason:
        return None, reason
    errors = document.get("errors")
    if document.get("contract") != "menura.compile_error" or not isinstance(errors, list) or not errors:
        return None, "not a compile error with a diagnostic"
    return errors[0], None


def check_misspelled_compile(responses):
    """Step 4: a misspelled metric gives `E_NAME_NOT_FOUND` with `alternatives` (PLN-5.4)."""
    error, reason = first_error(responses, COMPILE_MISSPELLED)
    if reason:
        return f"compile (misspelled metric): {reason}"
    if error.get("code") != "E_NAME_NOT_FOUND":
        return f"compile (misspelled metric): code is {error.get('code')!r}, expected 'E_NAME_NOT_FOUND'"
    if error.get("alternatives") != MISSPELLED_ALTERNATIVES:
        return f"compile (misspelled metric): alternatives are {error.get('alternatives')}, expected {MISSPELLED_ALTERNATIVES}"
    return None


def check_hidden_compile(responses):
    """Step 5: a metric the context may not use gives the error of a missing metric (PLN-5.4a)."""
    hidden, reason = first_error(responses, COMPILE_HIDDEN)
    if reason:
        return f"compile (hidden metric): {reason}"
    missing, reason = first_error(responses, COMPILE_MISSING)
    if reason:
        return f"compile (missing metric): {reason}"
    if hidden.get("code") != "E_NAME_NOT_FOUND":
        return f"compile (hidden metric): code is {hidden.get('code')!r}, expected 'E_NAME_NOT_FOUND'"
    as_missing = json.loads(json.dumps(hidden).replace(HIDDEN_METRIC, MISSING_METRIC))
    if as_missing != missing:
        return f"compile (hidden metric): {json.dumps(hidden, sort_keys=True)} differs from the missing metric's {json.dumps(missing, sort_keys=True)}"
    return None


def failures(responses):
    """One line per step that does not meet its expectation, in step order."""
    checks = [check_initialize, check_describe, check_valid_compile, check_misspelled_compile, check_hidden_compile]
    return [line for line in (check(responses) for check in checks) if line]


def run_server(binary, messages):
    """The standard output of `menura-mcp` after it reads every message and its input closes."""
    payload = "".join(json.dumps(message) + "\n" for message in messages)
    completed = subprocess.run(
        [str(binary)], input=payload, capture_output=True, text=True, encoding="utf-8", timeout=TIMEOUT_SECONDS
    )
    return completed.stdout


def main(arguments):
    """Run the fixed sequence and report; returns the exit status."""
    binary = pathlib.Path(arguments[0]) if arguments else DEFAULT_BINARY
    try:
        output = run_server(binary, requests(read_documents(MODEL_DIRECTORY)))
    except (OSError, subprocess.SubprocessError) as error:
        print(f"menura-mcp: cannot run {binary}: {error}")
        return 1
    lines = failures(responses_by_identifier(output))
    for line in lines:
        print(line)
    if lines:
        return 1
    print("agent-smoke: every step met its expectation")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
