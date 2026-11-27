# Menura

A semantic compiler: it turns a question in business terms (metrics, dimensions, filters) into
correct, secure, deterministic SQL. It reads semantic models in the **Apache Ossie** format,
speaks standard SQL, and never stores or reads data itself.

> Status: **1.1.1**, the ClickHouse dialect ([roadmap 7.2](spec/07-roadmap.md)). Contracts are
> frozen at major version 1 and follow [contract evolution, architecture 3.3](spec/architecture/README.md).

## What it does today

- Pure public functions over JSON contract documents ([contracts](spec/contracts/README.md)):
  `validate`, `load`, `compile`, `explain`, `describe`, `integrity_checks`, `dependencies`,
  `diff` and `migrate` ([architecture 3.5](spec/architecture/README.md)).
- Two target dialects, chosen with the `dialect` compile option: `duckdb` and `clickhouse`.
- Every query of the model and query contracts compiles (slice VS-1f): wide-table and SQL-form
  queries, several datasets and paths, derived, additivity, population and context metrics
  (period to date, prior period, share of total), evidence-based rewrites and summaries,
  dataset, field and row security, lineage, explanation and audit records.

## Command line

Every command writes its output document as JSON on standard output (ARC-5.4).

```sh
# Validate a model against a catalog schema (ARC-5.5).
menura validate --model model.yaml --catalog-schema catalog.json

# Write the model-dependency document (ARC-5.6).
menura dependencies --model model.yaml --catalog-schema catalog.json

# Diff the dependencies of two model versions (ARC-5.6).
menura diff --old-model old.yaml --old-catalog-schema old-catalog.json \
  --new-model new.yaml --new-catalog-schema new-catalog.json

# Write the migration report of a model (`menura.migration_report`).
menura migrate --model model.yaml
```

Each command also accepts `--load-options <path>` and `--help`.

**Audience (ARC-6.2).** The CLI serves modelers and developers and may show the whole compiled
plan, SQL and policy conditions included. End users and their agents never call Menura
directly: they query through the platform's semantic service.

## MCP server

`menura-mcp` speaks MCP over standard input and output, revision `2025-06-18` (ARC-6.6). It
opens no file or network connection: every document arrives as a tool argument. It has one tool
per CLI command, with the command's name (`-` written `_`): `validate`, `compile`, `explain`,
`describe`, `integrity_checks`, `dependencies`, `diff` and `migrate` (ARC-6.7). Each tool returns
the exact text the CLI writes for the same call.

## Python package

The Python package, imported as `menura`, is built with PyO3 as one `abi3` wheel per platform
for CPython 3.10 and later, with type stubs (ARC-6.3). Documents go in as `bytes` or `str` and
options as `dict` objects, read as the CLI reads its option files (ARC-6.4).

```python
import menura

handle = menura.Handle.load(model, catalog_schema)
result = handle.compile(query, security_context)
print(result.document, result.is_error)
```

## What 1.0.0 guarantees

From [roadmap 7.1b](spec/07-roadmap.md). What 1.0.0 does not support is rejected with an
error, never answered differently.

| Guaranteed | Not supported in 1.0.0 |
|---|---|
| Every Apache Ossie 0.1.1 model loads, with declared deviations DEV-1–DEV-9 | Development-line models outside `ossie_format: development` |
| Wide-table queries with full Layer 3 semantics and its suites passing | Predicates on metric values, subtotals, row listings, per-metric filters |
| SQL form: Layer 2 single-relation subset (E1-4), exact semantics | `JOIN`, `HAVING`, CTEs, subqueries, set operators, window functions in SQL |
| Additivity, population, derived and context metrics; summaries over integer and decimal sums | Window metrics, ranking, top N per group; summaries over floats, holistic or semi-additive metrics |
| Dataset, field and row security, deny by default | Value masking |
| DuckDB SQL, deterministic, session-independent | Other dialects |
| Contracts frozen at major version 1 | — |

## What 1.1.0 adds

The `clickhouse` dialect ([E6 ClickHouse](spec/stages/E6-clickhouse.md)) compiles the same plans
as DuckDB and returns the same rows; only the SQL text, the physical type names and the parameter
form differ. What ClickHouse cannot express fails with `E6-01`, never with an internal error or a
different answer:

- a relation `source` of three or more parts, since ClickHouse names a table with at most
  `database.table` (E6-7.1);
- a value of type `time`, `interval`, `timestamp` without time zone or a nullable `list`, and a
  `date` or `timestamp` parameter outside the range of its ClickHouse type (E6-7.12).

Everything 1.0.0 guarantees for DuckDB still holds.

## Supported platforms and channels

The Rust crate `core`, the CLI `menura` and the MCP server `menura-mcp` support Linux x86_64,
macOS x86_64, macOS arm64 and Windows x86_64, with the same output bytes on each
([platforms PLT-1](spec/architecture/platforms.md)). The Python package supports the platforms of
its wheels (ARC-6.3).

Support is claimed only for the rows of the compatibility matrix attached to each release
(`compatibility-matrix.md` and `compatibility-matrix.json`, CI-4). A combination the matrix does
not list is unverified, not supported ([continuous integration CI-4.4](spec/09-continuous-integration.md)).
This holds for each platform too, and for ClickHouse: only the platforms and the ClickHouse
release the matrix records are verified.

## Links

- Specification: [`spec/README.md`](spec/README.md)
- Agent instructions: [`AGENTS.md`](AGENTS.md)
- License: [Apache License 2.0](LICENSE)
