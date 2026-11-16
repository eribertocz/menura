# Contract: compiled plan and compile error

**Contract version:** 1.0.0 · **Owner:** `semantic_compiler` · **Producer:** compiler · **Consumers:** hosts, executors

`compile` returns exactly one document: a compiled plan, or a compile error.

## PLN-1 Compiled plan

| Field | Rule |
|---|---|
| `producer` | Compiler version, normative Apache Ossie release, tracked development commit. |
| `dialect` | Target dialect. Version 1: `duckdb`. |
| `data_source` | Logical source name. The executor maps it to a connection with its own credentials. |
| `sql` | One SQL statement in the target dialect. |
| `parameters` | Bound parameter values, by position (`$1`, `$2`, …), each with its logical type, encoded as CON-4 requires. |
| `result_schema` | Output columns in the query tree's column order (E1-5): dimensions then metrics for the wide-table form, `SELECT` order for the SQL form. Name, role, logical and physical type, nullability. |
| `cache_key` | See PLN-3. |
| `limits` | Suggested `timeout_ms` and `max_rows` (`null` means no limit beyond the query's own). |
| `lineage` | See PLN-4. |
| `explanation` | Every decision with its stage and reason (P7). |
| `warnings` | Non-blocking diagnostics. |

Schema: [`schemas/compiled-plan.schema.json`](schemas/compiled-plan.schema.json) · Example (illustrative SQL): [`examples/compiled-plan.example.json`](examples/compiled-plan.example.json)

## PLN-2 SQL rules

- **PLN-2.1** Every value that comes from the query or the security context is a bound parameter, except values from closed sets or already validated (E6-4.11). The SQL text contains only identifiers from the model and catalog, operators, constants written in the model, and those literals.
- **PLN-2.2** Identifiers are always quoted in the dialect's quoting style, exactly as the catalog spells them.
- **PLN-2.3** The statement is read only: a single `SELECT`, optionally preceded by `WITH`.
- **PLN-2.4** Output is byte-identical for identical inputs (P3), including whitespace and the order of every list.
- **PLN-2.5** The executor MUST NOT modify `sql` or `parameters`.

## PLN-3 Cache key

`key` is the SHA-256 hash of the RFC 8785 canonical JSON of `{plan_hash, security_fingerprint, data_versions}`.

- **PLN-3.1** `plan_hash` is the SHA-256 hash of the canonical JSON of `{dialect, data_source, sql, parameters, result_schema}`.
- **PLN-3.2** `security_fingerprint` is the security context fingerprint (SEC-3.6).
- **PLN-3.3** `data_versions` lists, from the catalog state, the version of every period read or, for relations that are not partitioned, the version of the whole relation (`period` is then `"*"`). For a summary, its versions are those of `aggregate_build.built_from`. It is `null` when the state document gives no versions for some relation read; a key with `null` versions MUST NOT be reused across data loads, so executors SHOULD NOT cache such results beyond their own time limit.

## PLN-4 Lineage

One entry per output column:

| Field | Content |
|---|---|
| `model_inputs` | Model fields and metrics the column derives from |
| `physical_inputs` | Catalog relations and columns read |
| `summary` | Summary dataset used, or `null` |
| `security_policies` | Row policies applied to the rows behind the column |

- **PLN-4.1** Lineage maps one-to-one onto the OpenLineage column-lineage facet: `physical_inputs` become input fields, `output` the output field.
- **PLN-4.2** Lineage never contains data values or attribute values.

## PLN-5 Compile error

| Field | Rule |
|---|---|
| `errors` | At least one diagnostic |
| `warnings` | Diagnostics gathered before the failure |

Each diagnostic has a `code`, an English `message` that names the offending item and the fix,
and a `location`: the input document and a JSON Pointer (RFC 6901), plus `line` and `column` for
YAML or SQL text when known.

- **PLN-5.1** The first error follows the precedence of E2-5 (queries) or E0-4.3 (models). Further errors MAY follow it.
- **PLN-5.2** A message never reveals data a user is not authorized to see. An unauthorized dataset, field or metric is reported exactly as a nonexistent one (`E_NAME_NOT_FOUND`).

Schema: [`schemas/compile-error.schema.json`](schemas/compile-error.schema.json) · Example: [`examples/compile-error.example.json`](examples/compile-error.example.json)

## PLN-6 Audit record

On request, `compile` also emits an audit record (schema: [`schemas/audit-record.schema.json`](schemas/audit-record.schema.json)):

- input hashes: model, catalog schema, catalog state, query, security fingerprint;
- `subject` from the security context;
- producer versions;
- outcome: `plan_hash`, or the error codes;
- every authorization decision (granted or denied, and by which rule) and every row policy applied;
- every decision of the explanation (`decisions`);
- the load options, including `grant_authenticated_to_all` (DEV-4), and the version of every input contract.

- **PLN-6.1** The record contains no data values and no attribute values.
- **PLN-6.2** Replaying the same inputs reproduces the same `plan_hash` (P3).
