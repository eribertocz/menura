# 3. Architecture

## 3.1 Independent modules

`menura` is one module of a larger system. Each module has its own repository and
specification, and communicates with the others **only through public contracts**.

| Module | Responsibility | Talks to the compiler through |
|---|---|---|
| Host (e.g. a BI application) | Orchestrates; authenticates users; presents results | Query, security context, compiled plan |
| Executor | Runs a compiled plan against a database; returns Apache Arrow | Compiled plan (read only) |
| Data pipelines | Load data; build summaries | Catalog state (write) |
| Modeling | Authors the semantic model | Apache Ossie document |
| Natural-language translator (after the MVP) | Turns a question into a query document | Model description in, query document out |
| Gateway (optional) | Exposes the SQL form over a standard network protocol for third-party tools | Query document in, compiled plan out, then the executor |

```
Pipelines ──► Catalog ─────┐
Modeling  ──► Model ───────┤
Host      ──► Query ───────┼──► menura ──► Compiled plan ──► Host ──► Executor ──► Database
Host      ──► Security ────┘
```

The compiler MUST NOT import code from any other module, open connections, or call other modules.

## 3.2 Contract ownership

Every contract has exactly one owner, which is the only party allowed to change it.

| Contract | Owner |
|---|---|
| Semantic model | Apache Ossie (extensions: `menura`) |
| Catalog schema and state | `menura` |
| Query | `menura` |
| Security context | `menura` |
| Compiled plan, explanation, errors | `menura` |

## 3.3 Contract evolution

1. Each contract is a JSON Schema (2020-12) with its own semantic version, independent of the compiler version.
2. A minor version only adds optional fields. A major version may break compatibility.
3. The compiler MUST accept the current and the previous major version during a declared transition period.
4. Every document MUST declare its contract version. An unknown version MUST produce an early error.
5. The owner publishes valid and invalid examples and a conformance suite that other modules run in their CI.
6. Nothing is removed without first being deprecated with a warning and a removal date.

## 3.4 Compiled plan

The compiled plan carries everything the executor needs without knowing any semantics:

- SQL with bound parameters (values are never concatenated into the text),
- dialect and logical data source,
- result schema (names, logical and physical types),
- cache key (plan hash, data versions used, security context fingerprint),
- timeout and row limit, which the executor enforces,
- explanation.

The executor MUST NOT modify the SQL. Its other obligations are in the [executor chapter](../contracts/executor.md).

## 3.5 Public functions

All functions are pure. Inputs and outputs are the contract documents of `spec/contracts/`.

| Function | Inputs | Output |
|---|---|---|
| `validate` | model, catalog schema, load options | Validation report: the same document as a compile error, with `errors` possibly empty (`menura.validation_report`) |
| `load` | model, catalog schema, optional catalog state, load options | Handle, with warnings; or compile error |
| `with_state` | handle, catalog state | New handle with the new state; the model is not reloaded |
| `integrity_checks` | handle, dialect | Integrity-check document ([contract](../contracts/integrity-checks.md)) |
| `compile` | handle, query, security context, compile options | Compiled plan, or compile error; with `audit: true`, also an audit record in both cases |
| `migrate` | model document, load options | Migration report: the model with deprecated forms rewritten, and each rewrite ([contract](../contracts/migration-report.md)) |
| `explain` | handle, query, security context, compile options | Exactly the document `compile` returns, with `sql` `""` and `parameters` `[]` (same schema, `contract` `menura.explanation`); the same compile error when `compile` fails; `audit` and `actor` are ignored (PLN-8) |
| `describe` | handle, security context | Model description |
| `dependencies` | handle | Model-dependency document ([contract](../contracts/model-dependencies.md)); for modelers and governance tools, never end users (DEP-3.1) |
| `diff` | two model-dependency documents | Model diff (DEP-4) |

| Load option | Meaning | Default |
|---|---|---|
| `model_name` | Which semantic model to load when the document has several (DEV-7) | — |
| `ossie_format` | `release` or `development` (section 4.5) | `release` |
| `grant_authenticated_to_all` | Treat datasets without `access` as `authenticated` (DEV-4); recorded in every plan | `false` |
| `namespace` | Extension namespace (MDL-2.3) | `menura` |
| `data_source` | Logical source name written in every plan | Catalog schema `document_id.origin` |

| Compile option | Meaning | Default |
|---|---|---|
| `dialect` | Target dialect: `duckdb`, or from 1.1.0 `clickhouse` (E6-7) | `duckdb` |
| `timeout_ms` | Written to `limits.timeout_ms` | 30000 |
| `max_rows` | Written to `limits.max_rows` | `null` |
| `audit` | Also return the audit record | `false` |
| `actor` | Delegated actor authenticated by the host: invocation metadata written only in the audit record, outside every hash (PLN-6.8) | absent |

- **ARC-5.1** A handle is immutable. A new model means `load`; a new catalog state means `with_state`.
- **ARC-5.2** The catalog state is validated by `load` and `with_state`. An invalid state produces a warning (`WCAT-01`–`WCAT-04`) and is ignored: the handle has no state, and E5 applies only rewrites that need no evidence. The state is admitted after the model, because CAT-8.8 needs the model's summaries, so its warning comes after the E0 warnings. A `WCAT-03` from CAT-8.8 is located at `/relations/<i>/aggregate_build` when the relation is not a summary, and at the offending period when its grain differs.

- **ARC-5.3** The conformance runner (chapter 8) is **test tooling, not part of the compiler**. It lives in its own crate, `conformance-runner`, which calls the public functions, reads case files and executes SQL on DuckDB. Like `cli`, it is exempt from A2 (I/O, clock, database). No other crate may depend on it, and it is never shipped as part of the compiler. `just conformance` runs it.
- **ARC-5.4** Every CLI command (section 3.6) writes its output document as JSON on standard output. Its exit status is `0` when the document is neither a compile error nor a validation report with errors (warnings do not change it); `1` when the document is a compile error or a validation report with at least one error; `2` when the command cannot run (usage error, an input file that cannot be read, or a file the command itself parses, such as the load-options file, that is not UTF-8), with a message on standard error and nothing on standard output. A command that cannot write its output to standard output also exits `2`, with a message on standard error; whatever it wrote before the failure is not an output document.
- **ARC-5.5** `menura validate --model <path> --catalog-schema <path> [--load-options <path>]`. The flags may come in any order, each at most once. `--load-options` names a JSON file holding an object with the load options of section 3.5; an option it omits takes its default. A missing required flag, an unknown or repeated flag, a positional argument, or a load-options file that is not such an object (an unknown option, or a value of the wrong type) is a usage error (exit `2`). The model and catalog schema files are read as bytes and passed to `validate` unchanged: their content problems are diagnostics in the report (exit `1`), never usage errors. `--help` writes the usage text to standard output and exits `0`, wherever it appears on the line. Only the form `--flag <value>` is accepted, so `--flag=value` is an unknown flag. A load option whose value is `null` has the wrong type.
- **ARC-5.6** `menura dependencies --model <path> --catalog-schema <path> [--load-options <path>]` loads the model as `validate` does and writes its model-dependency document. `menura diff --old-model <path> --old-catalog-schema <path> --new-model <path> --new-catalog-schema <path> [--load-options <path>]` loads both models with the same load options, old first, and writes the diff of their dependencies (DEP-4). A model that does not load gives its compile error (exit `1`); flags follow ARC-5.5.
- **ARC-5.7** `menura compile` and `menura explain` take `--model <path> --catalog-schema <path> --query <path> --security-context <path>` and the optional `--catalog-state <path>`, `--load-options <path>` and `--compile-options <path>`. `--compile-options` names a JSON file holding an object with the compile options of section 3.5, read as `--load-options` is (ARC-5.5). They load the model as `validate` does, call the function and write its document: the compiled plan or the explanation (exit `0`), or the compile error (exit `1`). With `audit: true`, `compile` also takes `--audit-record <path>` and writes there the audit record of the compilation, with the plan or the compile error; a model that does not load gives no record, since `compile` is not called. `audit: true` without `--audit-record`, or `--audit-record` without `audit: true`, is a usage error. A record that cannot be written exits `2`, with nothing on standard output. `explain` ignores `audit` and `actor` (PLN-8.3) and refuses `--audit-record` as an unknown flag.
- **ARC-5.8** `menura describe` takes `--model`, `--catalog-schema`, `--security-context` and the optional `--catalog-state` and `--load-options`, and writes the model description or the compile error. `menura integrity-checks` takes `--model`, `--catalog-schema` and the optional `--load-options` and `--dialect <name>` (default `duckdb`; another value is a usage error), and writes the integrity-check document or the compile error.
- **ARC-5.9** `menura migrate --model <path> [--load-options <path>]` writes the migration report (exit `0`) or the compile error (exit `1`). It reads no catalog file (MIG-1).
- **ARC-5.10** Every command follows ARC-5.4 and the flag rules of ARC-5.5. A document file named by a flag is read as bytes and passed unchanged; its content problems are diagnostics, never usage errors. The load-options and compile-options files are the exception: the CLI parses them, and their problems are usage errors (ARC-5.5, ARC-5.7).

## 3.6 Delivery forms

One core, written in Rust (reference implementation). Every other form is a thin wrapper without logic of its own:

| Form | Use |
|---|---|
| Rust crate | Native embedding |
| Python package | Embedding in Python hosts |
| CLI | `validate`, `compile`, `explain`, `describe`, `integrity-checks`, `dependencies`, `migrate`, `diff` |
| MCP server (`menura-mcp`, from 0.9.0) | One MCP tool per public function and per CLI command, for modelers and developers (ARC-6.1) |
| HTTP service (optional, after the MVP) | Same JSON contracts, for remote hosts. A Rust wrapper built on `axum` |

The public functions are those of section 3.5.

- **ARC-6.1** `menura-mcp` is a crate of this repository, a wrapper like `cli`, released with every Menura version from 0.9.0. It speaks MCP over standard input and output only, opens no network connection and has no logic of its own. Each CLI command has exactly one equivalent MCP tool (ARC-6.7), which takes the same inputs, applies the same validations and returns the same documents, errors and warnings. Like `cli`, it is exempt from A2 only for standard input and output.
- **ARC-6.2** **Audience boundary.** `menura-mcp` and the CLI serve modelers and developers (authoring), and may show the whole compiled plan, SQL, parameters and policy conditions included. End users and their agents never call Menura directly. They query through the platform's semantic service (Accipiter, an external module), which shows them the explanation without SQL, parameters or policy conditions. Menura's contracts give that service what it needs, and the boundary is enforced by the service and by deployment, not by the core (P2).

### 3.6.1 Python package

- **ARC-6.3** The Python package is the crate `python` of this repository, built with PyO3 as one `abi3` wheel per platform for GIL-enabled CPython 3.10 and later; free-threaded builds are not targeted before 1.0.0, because `abi3` does not cover them. Wheels cover Linux x86_64 and aarch64, macOS x86_64 and arm64, and Windows x86_64, and are built with maturin. The package ships type stubs (`.pyi`) and a `py.typed` marker, so its API is typed for checkers and editors. Its import name is `menura`. Among the workspace crates it depends only on `core` (A4); PyO3 is an external dependency approved in `deny.toml`. It has no logic of its own: each call converts its arguments, calls `core` and returns its documents. Q1 holds for it: if the pinned PyO3 cannot be used under `#![forbid(unsafe_code)]`, the implementation stops and reports a specification defect. Wheels are built and tested in CI; publishing them is a maintainer step.
- **ARC-6.4** **Arguments.** Documents go in as `bytes` or `str` (a `str` is encoded as UTF-8) and are passed to `core` unchanged, as the CLI passes its files (ARC-5.10): their content problems come back as documents. Options are `dict` objects converted as JSON and read as the CLI reads its option files (ARC-5.5): a `bool` is only a boolean, an `int` that is not a `bool` is an integer, and a `float` is the wrong type where an integer is required. An unknown option, a value of the wrong type, a `None` value, a key that is not a `str`, or an options argument that is not a `dict` raises `ValueError` with the CLI's usage message. An optional argument passed as `None` is absent.
- **ARC-6.5** **Results.** Every call returns a `Result` with `document`, the exact text the CLI writes to standard output for the same call without its final line feed, and `is_error`, true exactly when the CLI would exit `1` (ARC-5.4). Module functions: `validate(model, catalog_schema, load_options=None)`, `migrate(model, load_options=None)`, `diff(old_model, old_catalog_schema, new_model, new_catalog_schema, load_options=None)`, as `menura diff`. `Handle.load(model, catalog_schema, catalog_state=None, load_options=None)` returns an immutable `Handle` whose `warnings` is the JSON text of its load warnings, written as the `warnings` member of a compiled plan; `with_state` gives the load's E0 warnings followed by its `WCAT` warnings (ARC-5.2). It raises `menura.CompileError`, with the compile error as `document`, when the model does not load. `Handle` methods: `with_state(catalog_state)`, `compile(query, security_context, compile_options=None)`, `explain(…)`, `describe(security_context)`, `integrity_checks(dialect="duckdb")`, `dependencies()`. The `Result` of `compile` also has `audit_record`, the record's text when `audit` is true and `None` otherwise.
- **ARC-6.5a** **Threads.** A `Handle` may be shared between threads, and calls release the global interpreter lock while `core` runs. The `core` handle is therefore `Send` and `Sync`.

### 3.6.2 MCP server

- **ARC-6.6** **Transport and session.** `menura-mcp` is a binary that speaks MCP over standard input and output (ARC-6.1), revision `2025-06-18`, pinned per release. It writes nothing but protocol messages to standard output and opens no file or network connection: every document arrives as an argument. `initialize` answers with `protocolVersion` `2025-06-18` whatever the client asks for, `serverInfo` `{name: "menura-mcp", version: <Menura version>}` and `capabilities` `{tools: {listChanged: false}}`. Calls are answered independently, and a result never depends on the order of calls; a cancelled call may still be answered.
- **ARC-6.7** **Tools.** One tool per CLI command, with the command's name (`-` written `_`): `validate`, `compile`, `explain`, `describe`, `integrity_checks`, `dependencies`, `diff` and `migrate`. `load`, `with_state` and the `core` function `diff` over two dependency documents have no tool: every tool loads its models, as the CLI does. Each tool's input schema is an object with `additionalProperties: false` whose members are the command's flags without `--`, `-` written `_` (`model`, `catalog_schema`, `catalog_state`, `query`, `security_context`, `load_options`, `compile_options`, `dialect`, `old_model`, …): documents of type `string` holding the file's text, options of type `object`. `compile` with `audit: true` needs no `audit_record` member. An unknown tool, a missing or unknown member, a member of the wrong type, a `null` value, or any argument the CLI would refuse with a usage error is a JSON-RPC error `-32602` whose message is the CLI's usage reason with MCP member names (`model` for `--model`), without the CLI's usage text; an unknown tool's message is ``unknown tool `<name>` ``. Both wrappers render the same typed usage error of `core`.
- **ARC-6.8** **Results.** A tool returns one text content item, the exact text the CLI writes to standard output without its final line feed, and `isError: true` exactly when the CLI would exit `1`. When `audit` is true and `compile` ran, the audit record follows as a second text content item, with the plan or the compile error; a model that does not load gives no record (ARC-5.7). No `structuredContent` or `outputSchema` is used.
- **ARC-6.9** **Parity.** For every conformance case whose inputs are valid UTF-8, the documents of the MCP tools and of the Python package equal those of the CLI and of `core`, byte for byte (`conformance-wrappers`, CI-1); a case with input that is not UTF-8 cannot be passed as MCP text and is run through the other forms only. Tool names, input schemas and descriptions are listed in the crate's documentation; changing one follows section 3.3.

## 3.7 Integration

- **In-process:** the host imports the modules as libraries. This is the recommended mode.
- **Distributed:** the same modules run as services, with the same contracts.

A separate distribution repository assembles the modules. It holds a compatibility manifest (which
module and contract versions work together) and end-to-end tests that verify it.

## 3.8 Boundary enforcement (from the first commit)

Module boundaries are checked by the build, not left to discipline. Every rule below runs in CI
from the first commit, and a violation fails the build.

| ID | Rule | Check |
|---|---|---|
| A1 | The core MUST NOT depend on any crate or package of another module. | Dependency allowlist (`cargo deny`) |
| A2 | The core MUST NOT depend on network, filesystem, clock or database libraries. Only the wrappers `cli` and `menura-mcp` (F29, once it exists) and the test-tooling crate `conformance-runner` (ARC-5.3) are exempt. | Banned dependency list |
| A3 | Cross-module data MUST be a contract document validated against its JSON Schema. | Contract tests at every boundary |
| A4 | Wrappers (Python, CLI, HTTP) MUST contain no compilation logic. | Among the workspace crates, wrappers may depend only on `core`; their external dependencies are allowlisted in `deny.toml`. The conformance suite runs through each wrapper with identical results |
| A5 | Every contract change MUST bump its version and update its examples. | Schema diff check in CI |
| A6 | Any other module MUST be replaceable by a stub that only speaks the contract. | End-to-end tests run with stubs |

A module is considered independent only if it can be built, tested and released with every
other module absent.

## 3.9 Lineage and audit

Lineage and audit are compiler outputs, because only the compiler knows how each result was built.

**Lineage** is part of every compiled plan. It is column-level and reports, for each result column:

- the model fields, metrics and relationships it derives from,
- the physical tables and columns read, for its value, its filters, joins and row policies,
- the summary used, if any (why it was valid is in the explanation),
- the row policies applied (PLN-4).

The lineage format maps to the OpenLineage column-lineage facet, so external lineage tools can
consume it without translation.

**Audit record.** Every compilation can emit an audit record that contains no data values:

- input hashes (model, catalog state, query, security context fingerprint),
- versions (compiler, contracts, Apache Ossie),
- every decision with its reason (path, summary, rewrite),
- every security rule applied or denied,
- the plan hash and the outcome (plan or error code).

Because compilation is deterministic (P3), an audit record can be replayed to reproduce the exact
plan.
