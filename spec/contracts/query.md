# Contract: query

**Contract version:** 1.1.0 · **Owner:** `menura` · **Producers:** hosts, gateways, translators · **Consumer:** compiler

## QRY-1 Purpose and Apache Ossie status

A query asks for metrics, grouped by dimensions, under filters. It has two forms:

| Form | Field | Apache Ossie basis |
|---|---|---|
| Wide-table form | `query` | Layer 3 wide-table query interface, JSON binding |
| SQL form | `sql` | Layer 2 relational query interface with `AGG()`, exact semantics, version 1 subset (E1-4) |

- **QRY-1.1** Apache Ossie 0.1.1 defines the semantic model only. Layers 2 and 3 are normative at the tracked development commit (OSS-1.1).
- **QRY-1.2** Full Layer 2 conformance is not claimed until every Layer 2 suite case passes (OSS-2.5).

## QRY-2 Envelope

| Field | Required | Rule |
|---|---|---|
| `contract` | Yes | `menura.query` |
| `contract_version` | Yes | Semantic version, accepted as in CAT-3.2 |
| `time_zone` | Yes | IANA time zone name |
| `reference_time` | If the query needs "now" | RFC 3339 instant with offset. The compiler never reads a clock (P2). |
| `query` or `sql` | Exactly one | The question |

Envelope checks run before schema validation, in the order of E1-2.1.

## QRY-3 Wide-table form

The portable part is exactly the Apache Ossie Layer 3 JSON binding: `dimensions`, `metrics`, `filter`.

- **QRY-3.1** At least one dimension or metric (`E_EMPTY_AGGREGATION_QUERY`). `null` values and unknown keys are `E_INVALID_QUERY`.
- **QRY-3.2** References. A field is `dataset.field`. A model metric (every metric of a 0.1.1 model) is referenced by its bare name `metric`. A dataset metric (development-line models only) is `dataset.metric`.
- **QRY-3.3** All other Layer 3 rules apply as specified in stages E1–E4.

## QRY-4 Extensions

Capabilities Layer 3 lists as future options go in `nonportable_extensions`, one entry with
`vendor_name: COMMON` and `data` an object in the shape of MDL-2: `{"namespace": "menura", "version": 1, …}`.
Each is temporary (section 4.3). Another `namespace` is `E_INVALID_QUERY`; a deployment that configures another namespace (MDL-2.3) publishes its own schema with that value.

| Key | Rule |
|---|---|
| `time_grains` | **QRY-4.1** `[{dimension, grain}]`. The dimension MUST be in `dimensions`, at most one grain per dimension, and its field MUST be a time field (`EQRY-09`, `E2-01`). Grains: `day`, `week` (ISO, starting Monday), `month`, `quarter`, `year`, truncated in `time_zone`. |
| `date_range` | **QRY-4.2** `{dimension, from, to}` or `{dimension, relative: {unit, count, include_current}}`. The dimension need not be in `dimensions`. **Absolute:** `from` ≤ `to` (`EQRY-11`), both inclusive dates; for a `date` field the predicate is `from ≤ x ≤ to`; for a `timestamp with time zone` field it is `x ≥ from 00:00` and `x < (to + 1 day) 00:00`, both in `time_zone`; for a `timestamp` without time zone the same bounds are compared as wall-clock values, without conversion. **Relative:** `count` ≥ 1; the current unit is the unit containing `reference_time` in `time_zone`; with `include_current: true` (default) the range is the `count` units ending with the current unit, otherwise the `count` units ending with the previous unit; it is then an absolute range of whole units. |
| `paths` | **QRY-4.3** `[{from, to, relationships}]`: the relationships, in order, form a path from dataset `from` to dataset `to` (`EQRY-12` otherwise). The path is used whenever the lowering joins `to` from `from`. At most one entry per (`from`, `to`) (`EQRY-12`). |
| `filters` | **QRY-4.4** Additional filters, each following the rules of `filter`, combined with AND. |
| `order_by` | **QRY-4.5** `[{output, direction, nulls}]`; `output` MUST be an output name of the query (`EQRY-10`). Text is ordered by Unicode code point (binary collation, the DuckDB default); E6 never adds another collation. |
| `limit` | **QRY-4.6** Requires an `order_by` that includes every dimension output (`EQRY-04`). |

Out of version 1: predicates on metric values, subtotals, row listings, per-metric filters.

## QRY-5 SQL form

- **QRY-5.1** Each model dataset is exposed as the relation `<model name>.<dataset name>`.
- **QRY-5.2** A metric is exposed on relation `D`, and usable as `AGG(alias.metric)`, if it is a dataset metric of `D` (development-line models), or a model metric whose every component has home dataset `D` (DEV-8). Any other metric is not exposed (`E_NAME_NOT_FOUND`).
- **QRY-5.3** The version 1 subset and its semantics are in E1-4.

## QRY-6 Schema and examples

- Schema: [`schemas/query.schema.json`](schemas/query.schema.json)
- Examples: [`examples/query.json-form.example.json`](examples/query.json-form.example.json), [`examples/query.sql-form.example.json`](examples/query.sql-form.example.json)

## QRY-7 Errors

| Code | Condition |
|---|---|
| `E_INVALID_QUERY` | Not JSON, or fails the JSON Schema (Apache Ossie) |
| `E_EMPTY_AGGREGATION_QUERY` | No dimensions and no metrics (Apache Ossie) |
| `EQRY-01` | Both or neither of `query` and `sql` |
| `EQRY-02` | Unknown time zone |
| `EQRY-03` | The query needs `reference_time` and has none |
| `EQRY-04` | `limit` without a total `order_by` |
| `EQRY-05` | SQL construct outside the version 1 subset |
| `EQRY-06` | Unsupported extension `version` |
| `EQRY-08` | Unsupported `contract` or `contract_version` |
| `EQRY-09` | `time_grains` on a dimension not in `dimensions`, or two grains for one dimension |
| `EQRY-10` | `order_by` names an unknown output |
| `EQRY-11` | `date_range` with `from` after `to` |
| `EQRY-12` | `paths` relationships that do not chain from `from` to `to`, or two entries for the same pair |
| `EQRY-13` | SQL item that is not a plain field and has no alias |
| `WQRY-01` | Warning: a query or security context uses the former contract name `semantic_compiler.*`, or a query extension uses the former namespace (CON-0.5); read as the new name until 1.0.0 |
