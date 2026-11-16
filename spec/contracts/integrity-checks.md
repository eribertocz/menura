# Contract: integrity checks

**Contract version:** 1.0.0 · **Owner:** `semantic_compiler` · **Producer:** compiler (`integrity_checks`) · **Consumer:** data pipelines

## CHK-1 Purpose

Integrity checks turn assumptions into evidence. The compiler generates them; pipelines run them
after each load and report the results in the catalog state (`checks`, `aggregate_build.integrity_check`).

## CHK-2 Document

`{contract: "semantic_compiler.integrity_checks", contract_version, model_hash, dialect, checks: [...]}`; each check:

| Field | Meaning |
|---|---|
| `check_id` | Stable identifier: `<kind>:<target>` |
| `kind` | `key_uniqueness`, `orphan_keys`, `summary`, `population`, `semi_additive` |
| `target` | Relationship, dataset, summary or metric name |
| `definition_hash` | See CHK-3 |
| `sql`, `parameters` | Read-only query in the dialect; generated like any plan (E6) |
| `sources` | Catalog relations it reads, whose versions the result must report in `verified` |

- **CHK-2.1** A check passes when its query returns **zero rows**. Returned rows are the violations, for diagnosis.
- **CHK-2.2** The check queries see all rows: no row policy applies to them. They are run by the pipeline's identity, never by end users.

## CHK-3 Definition hash

The SHA-256 (CON-2) of the canonical JSON of `{kind, target, definition}`, where `definition`
is the normalized tree of every model element the check depends on: the relationship and the
keys and fields of both datasets (`key_uniqueness`, `orphan_keys`); the summary definition hash (`summary`); the population and metric
extensions (`population`, `semi_additive`).

- **CHK-3.1** A model change that affects a check changes its hash, so old evidence stops counting (CAT-7.3).

Schema: [`schemas/integrity-checks.schema.json`](schemas/integrity-checks.schema.json) · Example: [`examples/integrity-checks.example.json`](examples/integrity-checks.example.json)
