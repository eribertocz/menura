# Contract: catalog

**Contract version:** 1.0.0 · **Owner:** `semantic_compiler` · **Producer:** data pipelines · **Consumer:** compiler

## CAT-1 Purpose

The catalog is the compiler's only evidence about the physical database. The compiler never
queries the database (P2) and never assumes anything the catalog does not prove.

## CAT-2 Two documents

| Document | `contract` | Contains | Changes when | Used by |
|---|---|---|---|---|
| Schema | `semantic_compiler.catalog_schema` | Relations, columns, types | The database structure changes | E0 |
| State | `semantic_compiler.catalog_state` | Data versions per period, summary builds, integrity checks, row estimates | Data is loaded or corrected | E5 |

- **CAT-2.1** Each document has its own `document_id` and is versioned separately.
- **CAT-2.2** The state document MUST name, in `schema_document_id`, the schema document it is consistent with. If it differs from the schema used to load the model, the state document is ignored (`WCAT-04`).
- **CAT-2.3** E0 uses only the schema document; the state document never affects model validation.

## CAT-3 Format and validation

- JSON, validated with JSON Schema 2020-12:
  [`schemas/catalog-schema.schema.json`](schemas/catalog-schema.schema.json),
  [`schemas/catalog-state.schema.json`](schemas/catalog-state.schema.json).
- Valid examples: [`examples/`](examples/).
- **CAT-3.1** Validation has two levels: structural (the JSON Schema) and semantic (CAT-8).
- **CAT-3.2** `contract_version` is a semantic version. A document is accepted if its major version is known and its minor version is less than or equal to the latest known minor version.
- **CAT-3.3** All objects are closed (`additionalProperties: false`).

## CAT-4 Schema document

| Field | Rule |
|---|---|
| `engine` | Informative only. The compiler never interprets the engine name or version. |
| `relations[].source` | MUST be identical, character for character, to the `source` of the Apache Ossie dataset it describes. |
| `columns[].logical_type` | Neutral type. The only type the compiler reasons with. |
| `columns[].physical_type` | Engine type, kept for lineage only. Never parsed. |
| `columns[].nullable` | Whether the column admits NULL. |

### CAT-4.1 Logical types

| `kind` | Parameters | DuckDB example |
|---|---|---|
| `boolean` | — | `BOOLEAN` |
| `integer` | `bits` (8, 16, 32, 64, 128), `signed` | `INTEGER` → 32, signed |
| `decimal` | `precision` (1–38), `scale` (0–`precision`) | `DECIMAL(18,2)` |
| `float` | `bits` (32, 64) | `DOUBLE` → 64 |
| `text` | — | `VARCHAR` |
| `date` | — | `DATE` |
| `time` | — | `TIME` |
| `timestamp` | `precision` (`s`, `ms`, `us`, `ns`), `with_time_zone` | `TIMESTAMP WITH TIME ZONE` |
| `interval` | — | `INTERVAL` |
| `binary` | — | `BLOB` |
| `uuid` | — | `UUID` |
| `list` | `element` (a logical type) | `VARCHAR[]` |
| `unsupported` | — | Any type without a neutral equivalent |

- **CAT-4.2** Mapping a physical type to a logical type is the producer's duty and MUST be exact: precision is never rounded and types are never widened.
- **CAT-4.3** An `unsupported` column may exist; using it in a model expression is an error in E0.

### CAT-4.4 DuckDB and DuckLake mapping

- **CAT-4.4** A producer for DuckDB, or for DuckLake (whose column types are DuckDB's), maps physical types exactly as this table states. Every type the table does not list is `unsupported`.

| DuckDB type | Logical type |
|---|---|
| `BOOLEAN` | `boolean` |
| `TINYINT`, `SMALLINT`, `INTEGER`, `BIGINT`, `HUGEINT` | `integer`, `bits` 8, 16, 32, 64, 128, `signed: true` |
| `UTINYINT`, `USMALLINT`, `UINTEGER`, `UBIGINT`, `UHUGEINT` | `integer`, `bits` 8, 16, 32, 64, 128, `signed: false` |
| `DECIMAL(p,s)` | `decimal`, `precision` p, `scale` s |
| `FLOAT` | `float`, `bits` 32 |
| `DOUBLE` | `float`, `bits` 64 |
| `VARCHAR` | `text` |
| `BLOB` | `binary` |
| `UUID` | `uuid` |
| `DATE` | `date` |
| `TIME` | `time` |
| `INTERVAL` | `interval` |
| `TIMESTAMP_S`, `TIMESTAMP_MS`, `TIMESTAMP`, `TIMESTAMP_NS` | `timestamp`, `precision` `s`, `ms`, `us`, `ns`, `with_time_zone: false` |
| `TIMESTAMP WITH TIME ZONE` (`TIMESTAMPTZ`) | `timestamp`, `precision` `us`, `with_time_zone: true` |
| `T[]` (`LIST`) where `T` maps to a type other than `unsupported` | `list`, `element` the mapping of `T` |
| `STRUCT`, `MAP`, `UNION`, fixed-size `ARRAY`, `ENUM`, `BIT`, `TIMETZ`, `VARINT`, `JSON`, and a `LIST` of an `unsupported` type | `unsupported` |

`ENUM` sorts by declaration order, not as text, and `JSON` has its own comparison functions. Neither has an exact neutral equivalent, so a producer that wants them queried casts them to `VARCHAR` in the relation it publishes.

## CAT-5 State document

| Field | Purpose |
|---|---|
| `row_count_estimate` | Estimate used only to choose among valid summaries. Never affects correctness. |
| `version` | On relations that are not partitioned: the version of the whole relation. Each relation has exactly one of `version`, `partitions` (base relations) or `aggregate_build` (summaries). |
| `partitions` | On partitioned relations: the data version of each period, for one column and one grain. |
| `aggregate_build` | On summaries: definition hash, coverage, the base versions each period was built from, the time zone, and the last integrity check. |
| `checks` | Results of the integrity checks generated by `integrity_checks` ([contract](integrity-checks.md)), other than summary checks, with the data versions they were run on. |

### CAT-5.1 Canonical periods

| Grain | Form | Example |
|---|---|---|
| `day` | `YYYY-MM-DD` | `2026-10-31` |
| `week` | `YYYY-Www`: ISO 8601 week-numbering year and week (2021-01-03 is `2020-W53`) | `2026-W44` |
| `month` | `YYYY-MM` | `2026-10` |
| `quarter` | `YYYY-Qn` | `2026-Q4` |
| `year` | `YYYY` | `2026` |

## CAT-6 Data versions are opaque tokens

A version is `{origin, snapshot}`: `origin` identifies the source, and `snapshot` identifies an
immutable state of it.

- **CAT-6.1** Tokens are compared by equality of both fields only. They are never ordered or parsed, even when they look numeric.

## CAT-7 Freshness

### CAT-7.1 Producer guarantees

| ID | Guarantee |
|---|---|
| CAT-7.1.1 | A token is never reused for different data. |
| CAT-7.1.2 | `schema_document_id` names exactly the schema the state is consistent with. |
| CAT-7.1.3 | `aggregate_build.definition_hash` is the hash the compiler computed for the definition the summary was built from: the `definition_hash` of the summary's `summary` check (CHK-3), copied unchanged. `aggregate_build.integrity_check.definition_hash` is the same value. |
| CAT-7.1.4 | `integrity_check` reports the result of the check query the compiler generated for that same hash, run on the versions listed in `verified`. |
| CAT-7.1.5 | `partitions.versions` lists every period of the relation that contains rows. |

### CAT-7.2 Summary validity

Let R be a summary of base relation B, and P the periods of the queried range at R's grain. Without a date range on R's coverage dimension, P is every period listed in B's `partitions` (CAT-7.1.5).
R is valid only if **all** of the following hold:

1. B has `partitions` on the column R's coverage dimension comes from, at R's grain. If that Apache Ossie field is an expression rather than a plain column, there is no evidence and R is not used.
2. Every period of P is inside `coverage`.
3. For every period of P, B's version in `partitions` exists and equals R's version in `built_from`.
4. R's `definition_hash` equals the hash of the current definition.
5. `aggregate_build.time_zone` equals the query's time zone.
6. `integrity_check.status` is `passed`, its `definition_hash` equals the current summary definition hash, and for every period of P its `verified` version equals B's current version.

If any datum is missing or any condition fails, R is not used and the base relation is queried.
The explanation states which condition failed.

### CAT-7.3 Check evidence

A check in `checks` is evidence only if its `status` is `passed`, its `definition_hash` equals
the current check's hash, and `verified` lists, for every relation involved, the current version
of every one of its periods (or its single version when it is not partitioned), with no
difference. Only then may E5 use it: skip deduplication (`key_uniqueness`) or use an inner join
(`orphan_keys`).

### CAT-7.4 Never

- Interpret an engine version or token content.
- Assume freshness without catalog evidence.
- Use `row_count_estimate` to decide correctness.

## CAT-8 Semantic rules

1. `source` is unique within each document.
2. Column names are unique within a relation after normalization as an unquoted identifier (E0D-1.1), so a regular reference to a column is never ambiguous (E0D-1.5).
3. For `decimal`, `scale` ≤ `precision`.
4. Every `source` in the state exists in the referenced schema document.
5. `partitions.column` exists in the relation and is of kind `date` or `timestamp`.
6. Every period is in the canonical form of its grain and is not repeated in its list. Where the document declares no grain (`aggregate_build`, `checks[].verified`), a period has the grain its shape denotes: `YYYY` year, `YYYY-Qn` quarter, `YYYY-MM` month, `YYYY-Www` ISO week, `YYYY-MM-DD` day; it must name a real calendar date or ISO week. All periods of one `aggregate_build` have the same grain, which CAT-8.8 compares with the summary's grain. In `checks[].verified` the pair of `source` and period is not repeated.
7. `coverage.from` ≤ `coverage.to`.
8. `aggregate_build` appears only on relations the model declares as summaries; otherwise the state is ignored (`WCAT-03`), checked by `load` and `with_state`.
9. For a partitioned relation, `partitions.versions` lists every period that contains data; a period not listed has no rows (producer guarantee CAT-7.1.5).

## CAT-9 Errors

| Code | Condition | Effect |
|---|---|---|
| `ECAT-01` | The schema document does not satisfy its JSON Schema | `load` fails |
| `ECAT-02` | Schema document: `contract` or `contract_version` not supported | `load` fails |
| `ECAT-03` | Schema document violates a CAT-8 rule | `load` fails |
| `WCAT-01` | The state document does not satisfy its JSON Schema | State ignored |
| `WCAT-02` | State document: `contract` or `contract_version` not supported | State ignored |
| `WCAT-03` | State document violates a CAT-8 rule | State ignored |
| `WCAT-04` | `schema_document_id` does not match the schema in use | State ignored |

- **CAT-9.1** Order of checks for each document: the document is JSON, then `contract` and `contract_version` (`ECAT-02` / `WCAT-02`), then the JSON Schema (`ECAT-01` / `WCAT-01`), then, for the state document, `schema_document_id` (`WCAT-04`), then the CAT-8 rules (`ECAT-03` / `WCAT-03`). The CAT-8 rules of a state document are therefore always checked against the schema it refers to. The first failure is reported, as in E1-2.1.

A rejected state document never prevents compilation: without evidence, E5 always uses base
relations, and the explanation says so.
