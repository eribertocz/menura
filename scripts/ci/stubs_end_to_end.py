"""The `stubs` check (CI-1, A6): end-to-end run of the release binary between two stub modules.

The stubs know only the published contracts (CON-0.1, CON-0.2), never the compiler's code:

- the catalog stub writes a `menura.catalog_schema` and a `menura.catalog_state` document for
  the tables of the `sample` conformance model;
- the executor stub reads a `menura.compiled_plan` and checks the members it would use: `sql`,
  `parameters` (ordered, typed), `result_schema` and `cache_key`. It runs the SQL with its
  parameters on DuckDB when the `duckdb` Python package is installed; otherwise it validates the
  plan against its contract only.

`menura compile` compiles a fixed query between them, and the executor stub also consumes the
`menura describe` output. Every document is validated against its schema in
`spec/contracts/schemas/` with the validator of the `examples` job (`jsonschema`, Draft 2020-12).
The stubs themselves use the standard library only.

Usage: python3 -I scripts/ci/stubs_end_to_end.py <path to the menura binary>
"""
import hashlib
import importlib.util
import json
import pathlib
import re
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
SCHEMAS = ROOT / "spec" / "contracts" / "schemas"
MODEL = ROOT / "conformance" / "models" / "sample"
HASH = re.compile(r"^[0-9a-f]{64}$")
PLACEHOLDER = re.compile(r"\$([0-9]+)")
ORIGIN = "stub:catalog"

SCHEMA_FILES = {
    "menura.catalog_schema": "catalog-schema",
    "menura.catalog_state": "catalog-state",
    "menura.query": "query",
    "menura.security_context": "security-context",
    "menura.compiled_plan": "compiled-plan",
    "menura.model_description": "model-description",
}

# The relations the catalog stub publishes: (source, [(column, logical type, physical, nullable)]).
INTEGER = {"kind": "integer", "bits": 32, "signed": True}
TEXT = {"kind": "text"}
RELATIONS = [
    ("shop.orders", [
        ("order_id", INTEGER, "INTEGER", False),
        ("status", TEXT, "VARCHAR", False),
        ("amount", INTEGER, "INTEGER", False),
    ]),
    ("shop.balances", [
        ("branch", TEXT, "VARCHAR", False),
        ("customer_id", INTEGER, "INTEGER", False),
        ("balance", INTEGER, "INTEGER", True),
    ]),
]

QUERY = {
    "contract": "menura.query",
    "contract_version": "1.1.0",
    "reference_time": "2026-11-15T12:00:00-04:00",
    "time_zone": "UTC",
    "query": {
        "dimensions": ["orders.status"],
        "metrics": [{"metric": "total_amount", "alias": "total_amount"}],
        "filter": "orders.status = 'OPEN'",
    },
}
EXPECTED_ROWS = [("OPEN", 15)]


class StubError(Exception):
    """A stub refused a document; the message says which member and why."""


def catalog_schema():
    """The catalog stub's `menura.catalog_schema` document."""
    return {
        "contract": "menura.catalog_schema",
        "contract_version": "1.1.0",
        "document_id": {"origin": ORIGIN, "snapshot": "schema-1"},
        "engine": {"name": "duckdb", "version": "2.0.0"},
        "relations": [
            {
                "source": source,
                "columns": [
                    {"name": name, "logical_type": logical, "physical_type": physical, "nullable": nullable}
                    for name, logical, physical, nullable in columns
                ],
            }
            for source, columns in RELATIONS
        ],
    }


def catalog_state():
    """The catalog stub's `menura.catalog_state` document: one version per relation."""
    return {
        "contract": "menura.catalog_state",
        "contract_version": "1.1.0",
        "document_id": {"origin": ORIGIN, "snapshot": "state-1"},
        "schema_document_id": {"origin": ORIGIN, "snapshot": "schema-1"},
        "relations": [
            {"source": source, "row_count_estimate": 3, "version": {"origin": ORIGIN, "snapshot": f"{index}"}}
            for index, (source, _) in enumerate(RELATIONS, start=1)
        ],
    }


def canonical_json(value):
    """RFC 8785 canonical JSON of strings, integers, booleans, null, arrays and objects (CON-2)."""
    if isinstance(value, float):
        raise StubError("the executor stub does not canonicalize floats")
    if isinstance(value, dict):
        members = sorted(value.items(), key=lambda item: item[0].encode("utf-16-be"))
        return "{" + ",".join(f"{canonical_json(key)}:{canonical_json(member)}" for key, member in members) + "}"
    if isinstance(value, list):
        return "[" + ",".join(canonical_json(item) for item in value) + "]"
    return json.dumps(value, ensure_ascii=False)


def sha256_of(value):
    """The SHA-256 hash of the canonical JSON of `value`, as 64 lowercase hex digits."""
    return hashlib.sha256(canonical_json(value).encode("utf-8")).hexdigest()


def require(document, member, kind):
    """The member `member` of `document`, which must be present with type `kind`."""
    if member not in document:
        raise StubError(f"the plan has no `{member}`")
    value = document[member]
    if not isinstance(value, kind) or isinstance(value, bool):
        raise StubError(f"`{member}` is not a {kind.__name__}")
    return value


def check_parameters(plan):
    """The plan's parameters, checked ordered by position from 1, typed, and used by the SQL."""
    sql = require(plan, "sql", str)
    parameters = require(plan, "parameters", list)
    for index, parameter in enumerate(parameters, start=1):
        if not isinstance(parameter, dict) or parameter.get("position") != index:
            raise StubError(f"parameter {index} is out of order")
        if not isinstance(parameter.get("logical_type"), dict) or "kind" not in parameter["logical_type"]:
            raise StubError(f"parameter {index} has no logical type")
        if "value" not in parameter:
            raise StubError(f"parameter {index} has no value")
    used = {int(position) for position in PLACEHOLDER.findall(sql)}
    if used != set(range(1, len(parameters) + 1)):
        raise StubError("the SQL placeholders do not match the parameters")
    return parameters


def check_result_schema(plan):
    """The plan's result schema: named, typed columns with unique names."""
    columns = require(plan, "result_schema", list)
    names = [column.get("name") for column in columns if isinstance(column, dict)]
    if not columns or len(names) != len(columns) or len(set(names)) != len(names):
        raise StubError("`result_schema` does not name every column once")
    for column in columns:
        if not isinstance(column.get("logical_type"), dict) or not isinstance(column.get("physical_type"), str):
            raise StubError(f"column `{column['name']}` is not typed")
    return columns


def check_cache_key(plan):
    """The plan's cache key, recomputed from the plan as PLN-3 and PLN-3.1 define it."""
    cache_key = require(plan, "cache_key", dict)
    for member in ("key", "plan_hash", "security_fingerprint"):
        if not isinstance(cache_key.get(member), str) or not HASH.match(cache_key[member]):
            raise StubError(f"`cache_key.{member}` is not a SHA-256 hash")
    if "data_versions" not in cache_key:
        raise StubError("`cache_key` has no `data_versions`")
    hashed = {member: plan[member] for member in ("dialect", "data_source", "sql", "parameters", "result_schema")}
    if sha256_of(hashed) != cache_key["plan_hash"]:
        raise StubError("`cache_key.plan_hash` does not match the plan")
    keyed = {member: cache_key[member] for member in ("plan_hash", "security_fingerprint", "data_versions")}
    if sha256_of(keyed) != cache_key["key"]:
        raise StubError("`cache_key.key` does not match its members")
    return cache_key


def check_plan(plan, description):
    """The executor stub's checks of a compiled plan and the model description of its context."""
    if plan.get("contract") != "menura.compiled_plan":
        raise StubError("the document is not a compiled plan")
    if plan.get("dialect") != "duckdb":
        raise StubError("the executor stub runs only DuckDB plans")
    parameters = check_parameters(plan)
    columns = check_result_schema(plan)
    cache_key = check_cache_key(plan)
    if description.get("security_fingerprint") != cache_key["security_fingerprint"]:
        raise StubError("the description and the plan are for different security contexts")
    return parameters, columns


def execute(plan, rows_sql):
    """The plan's rows on DuckDB, with its parameters bound by position (PLN-2.5)."""
    import duckdb  # Only when installed: see `duckdb_available`.

    connection = duckdb.connect()
    for statement in rows_sql.split(";"):
        if statement.strip():
            connection.execute(statement)
    values = [parameter["value"] for parameter in plan["parameters"]]
    return [tuple(row) for row in connection.execute(plan["sql"], values).fetchall()]


def duckdb_available():
    """Whether the `duckdb` Python package is installed."""
    return importlib.util.find_spec("duckdb") is not None


def validate(document):
    """Every schema error of `document` against the schema of its `contract`."""
    import jsonschema  # The validator of the `examples` job, installed by the `stubs` job.

    name = SCHEMA_FILES[document["contract"]]
    schema = json.loads((SCHEMAS / f"{name}.schema.json").read_text(encoding="utf-8"))
    validator = jsonschema.Draft202012Validator(schema, format_checker=jsonschema.FormatChecker())
    return [f"{name}: {error.json_path}: {error.message}" for error in validator.iter_errors(document)]


def write(directory, name, document):
    """Writes `document` as JSON under `directory` and returns its path."""
    path = directory / name
    path.write_text(json.dumps(document, indent=2), encoding="utf-8")
    return path


def run_menura(binary, arguments):
    """The JSON document `menura` writes on standard output; fails unless it exits 0."""
    completed = subprocess.run([binary, *arguments], capture_output=True, text=True, check=False)
    if completed.returncode != 0:
        raise StubError(f"menura {arguments[0]} exited {completed.returncode}: {completed.stdout}{completed.stderr}")
    return json.loads(completed.stdout)


def main(arguments):
    if len(arguments) != 1:
        print(__doc__)
        return 2
    binary = arguments[0]
    context = json.loads((MODEL / "contexts" / "analyst.json").read_text(encoding="utf-8"))
    with tempfile.TemporaryDirectory() as temporary:
        directory = pathlib.Path(temporary)
        inputs = {
            "schema": write(directory, "catalog-schema.json", catalog_schema()),
            "state": write(directory, "catalog-state.json", catalog_state()),
            "query": write(directory, "query.json", QUERY),
            "context": write(directory, "security-context.json", context),
        }
        common = ["--model", str(MODEL / "model.yaml"), "--catalog-schema", str(inputs["schema"]),
                  "--catalog-state", str(inputs["state"]), "--security-context", str(inputs["context"])]
        plan = run_menura(binary, ["compile", *common, "--query", str(inputs["query"])])
        description = run_menura(binary, ["describe", *common])
    documents = [catalog_schema(), catalog_state(), QUERY, context, plan, description]
    failures = [failure for document in documents for failure in validate(document)]
    for failure in failures:
        print(failure)
    if failures:
        return 1
    check_plan(plan, description)
    if plan["cache_key"]["data_versions"] is None:
        raise StubError("the catalog stub's state gives every version, yet `data_versions` is null")
    if duckdb_available():
        rows = execute(plan, (MODEL / "data.sql").read_text(encoding="utf-8"))
        if rows != EXPECTED_ROWS:
            raise StubError(f"DuckDB returned {rows}, expected {EXPECTED_ROWS}")
        print("stubs: the plan ran on DuckDB with its parameters")
    else:
        print("stubs: duckdb is not installed; the plan was validated against its contract only")
    print("stubs: every document is valid and the executor stub accepted the plan")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except StubError as error:
        print(f"stubs: {error}")
        sys.exit(1)
