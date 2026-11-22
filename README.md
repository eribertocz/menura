# Menura

A semantic compiler: it turns a question in business terms (metrics, dimensions, filters) into
correct, secure, deterministic SQL. It reads semantic models in the **Apache Ossie** format,
speaks standard SQL, and never stores or reads data itself.

> Status: pre-1.0. Current release line: **0.8.0**. Before 1.0.0, any minor release may change
> contracts ([roadmap](spec/07-roadmap.md)).

## What it does today

- Pure public functions over JSON contract documents ([contracts](spec/contracts/README.md)):
  `validate`, `load`, `compile`, `explain`, `describe`, `integrity_checks`, `dependencies` and
  `diff` ([architecture 3.5](spec/architecture/README.md)).
- One target dialect: DuckDB.
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
```

Each command also accepts `--load-options <path>` and `--help`.

**Audience (ARC-6.2).** The CLI serves modelers and developers and may show the whole compiled
plan, SQL and policy conditions included. End users and their agents never call Menura
directly: they query through the platform's semantic service.

## What comes next

- 0.9.0: Python package, complete CLI (`migrate` included), the `menura-mcp` server, performance
  targets. Release candidate.
- 1.0.0: the MVP, with contracts frozen at major version 1.

## Links

- Specification: [`spec/README.md`](spec/README.md)
- Agent instructions: [`AGENTS.md`](AGENTS.md)
- License: [Apache License 2.0](LICENSE)
