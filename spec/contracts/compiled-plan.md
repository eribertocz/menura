# Contract: compiled plan and compile error

**Contract version:** 1.2.0 · **Owner:** `menura` · **Producer:** compiler · **Consumers:** hosts, executors

`compile` returns exactly one document: a compiled plan, or a compile error.

## PLN-1 Compiled plan

| Field | Rule |
|---|---|
| `producer` | Compiler version, normative Apache Ossie release, tracked development commit. |
| `dialect` | Target dialect. Version 1: `duckdb`. |
| `data_source` | Logical source name. The executor maps it to a connection with its own credentials. |
| `sql` | One SQL statement in the target dialect. |
| `parameters` | Bound parameter values, by position (`$1`, `$2`, …), each with its logical type, encoded as CON-4 requires. |
| `result_schema` | Output columns in the query tree's column order (E1-5): dimensions then metrics for the wide-table form, `SELECT` order for the SQL form. Name, role, logical and physical type, nullability. `nullable` is true for every column that comes from the right side of a left `Join`, and for a stitched component unless both its values and its empty-input value are non-NULL. Every other column keeps the nullability of its expression. |
| `cache_key` | See PLN-3. |
| `limits` | Suggested `timeout_ms` and `max_rows` (`null` means no limit beyond the query's own). |
| `lineage` | See PLN-4. |
| `explanation` | Every decision with its stage and reason (P7). See PLN-7. |
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
- **PLN-3.3** `data_versions` lists, from the catalog state, the version of every period read or, for relations that are not partitioned, the version of the whole relation (`period` is then `"*"`). For a summary, its versions are those of `aggregate_build.built_from`. It is `null` when the state document gives no versions for some relation read; a key with `null` versions MUST NOT be reused across data loads, so executors SHOULD NOT cache such results beyond their own time limit. Entries are ordered by relation `source`, then by `period`, comparing code points.
- **PLN-3.3a** A period is read when its relation is scanned: every `Scan` of the plan reads every period of its relation, whether the scan serves a value, a join, a row policy or a propagation `SemiJoin`. Before 1.0.0 a `date_range` prunes no period, so `data_versions` lists every period the catalog state gives for each relation scanned.

## PLN-4 Lineage

One entry per output column:

| Field | Content |
|---|---|
| `output` | The column's `result_schema` name |
| `model_inputs` | Model fields, metrics and relationships the column derives from (PLN-4.4) |
| `physical_inputs` | Catalog relations and columns read (PLN-4.5) |
| `summary` | Summary dataset used, or `null` (PLN-4.6) |
| `security_policies` | Row policies applied to the rows behind the column (PLN-4.7) |

- **PLN-4.1** Lineage maps one-to-one onto the OpenLineage column-lineage facet: `physical_inputs` become input fields, `output` the output field.
- **PLN-4.2** Plan outputs are host-facing (SCY-4a.1). Lineage and the explanation (PLN-7) may name summaries, row policies and datasets joined only to apply a policy (E4-6.10), but never contain data values, attribute values or the literal values of the query: filter literals, `date_range` bounds and `reference_time`.
- **PLN-4.3** `lineage` has exactly one entry per `result_schema` column, in `result_schema` order, and `output` is that column's name.
- **PLN-4.4** `model_inputs` lists every model element the column's value derives from, transitively: the metric itself and every metric it reads (derived and context metrics included), every field those metrics or the dimension read, directly or through other fields, and every relationship of the `Join` nodes that bring one of those fields to the home dataset of a branch behind the column. Entries are written `field:<dataset>.<field>`, `metric:<name>` and `relationship:<name>`, with names as the model declares them, without duplicates, ordered by kind (`field`, `metric`, `relationship`), then in CON-5 order of the normalized name (for a field, the dataset name, then the field name).
- **PLN-4.5** `physical_inputs` lists every catalog column read to compute the column: the columns its value reads (population and carry-forward reads included, E2-4.9), and the columns read by the query filter, the `date_range` predicate, the `Join` conditions and the row-policy `Filter` and `SemiJoin` nodes of every branch behind the column. `source` is the relation's catalog `source` and `column` the column as the catalog spells it. A branch served by a summary (E5-3.5) lists the columns of the summary relation it reads, not those of the base relation it replaces. Entries have no duplicates and are ordered by `source`, then by `column`, comparing code points.
- **PLN-4.6** `summary` is the summary dataset that serves a branch behind the column, or `null` when none does. When several such branches use summaries, it names the summary of the first of them in branch order (E3-4.3).
- **PLN-4.7** `security_policies` lists every row policy applied to the rows behind the column, propagated policies included (E4-6.5), each written `<dataset>.<policy>`, where `<dataset>` is the dataset that declares it. Entries have no duplicates and are ordered in CON-5 order of the normalized dataset name, then of the normalized policy name.
- **PLN-4.8** The branches behind a column are, for a dimension, every branch of the plan and, for a metric, every branch that computes one of its components.
- **PLN-4.9** Why a summary was valid is not part of lineage: the explanation records it (PLN-7.3).

## PLN-5 Compile error

| Field | Rule |
|---|---|
| `errors` | At least one diagnostic |
| `warnings` | Diagnostics gathered before the failure |

Each diagnostic has a `code`, an English `message` that names the offending item and the fix,
and a `location`: the input document and a JSON Pointer (RFC 6901), plus `line` and `column` for
YAML or SQL text when known. An `E_NAME_NOT_FOUND` diagnostic may also carry `alternatives`
(PLN-5.4).

- **PLN-5.1** The first error follows the precedence of E2-5 (queries) or E0-4.3 (models). Further errors MAY follow it.
- **PLN-5.1a** A compiler defect that no specification code describes (an internal invariant that failed) is reported as a compile error with code `E_INTERNAL`, `location` `null`, and a message naming the stage. It is never expected: a conformance case or a fuzzing run that produces it is a failure.
- **PLN-5.2** A message never reveals data a user is not authorized to see. An unauthorized dataset, field or metric is reported exactly as a nonexistent one (`E_NAME_NOT_FOUND`).
- **PLN-5.3** `validate` returns the same document with `contract` `menura.validation_report`, where `errors` may be empty. It shares this schema and is not a separate contract. Example: [`examples/validation-report.example.json`](examples/validation-report.example.json).
- **PLN-5.4** **Alternatives.** An `E_NAME_NOT_FOUND` diagnostic of `compile` or `explain` carries `alternatives`, the names the user may write instead, built as follows. No other diagnostic carries the member, and it is omitted when the list is empty.
  1. The unknown reference has the kind its position in the query expects: a field (`<dataset>.<field>`), a metric, or a dataset (a `paths` entry).
  2. Candidates are the objects of that kind the user is authorized to see: fields authorized by E2-4.2 on datasets of the authorized subgraph (E2-4.8), metrics authorized by E2-4.3, and datasets of the authorized subgraph. Summaries are never candidates (E2-4.6).
  3. A candidate is kept when the Levenshtein distance between its normalized name (E0D-1; for a field, `<dataset>.<field>`) and the normalized reference as written, counted in Unicode code points, is at most 2, and its written entry is at most 256 code points long.
  4. Kept candidates are ordered by distance, then in CON-5 order of the normalized name; at most the first 5 are written, each as `field:<dataset>.<field>`, `metric:<name>` or `dataset:<name>`, with the names as the model declares them.
- **PLN-5.4a** Candidates come only from authorized objects, so a reference to a hidden object and a reference to a nonexistent one with the same spelling get the same `alternatives`, as they get the same message (PLN-5.2).

Schema: [`schemas/compile-error.schema.json`](schemas/compile-error.schema.json) · Example: [`examples/compile-error.example.json`](examples/compile-error.example.json)

## PLN-6 Audit record

On request, `compile` also emits an audit record (schema: [`schemas/audit-record.schema.json`](schemas/audit-record.schema.json) · Example: [`examples/audit-record.example.json`](examples/audit-record.example.json)):

- input hashes: model, catalog schema, catalog state, query, security fingerprint;
- `subject` from the security context;
- producer versions;
- outcome: `plan_hash`, or the error codes;
- every authorization decision (granted or denied, and by which rule) and every row policy applied;
- every decision of the explanation (`decisions`);
- the load options, including `grant_authenticated_to_all` (DEV-4), the compile options that shape the plan, and the version of every input contract;
- the delegated `actor`, when the host passes one.

- **PLN-6.1** The record contains no data values and no attribute values.
- **PLN-6.2** Replaying the same inputs reproduces the same `plan_hash` (P3).
- **PLN-6.3** **Input hashes.** `inputs.model` is the model hash (E0H-3.2). `inputs.catalog_schema`, `inputs.catalog_state` and `inputs.query` are the SHA-256 hash of the RFC 8785 canonical JSON of the document as passed, or of its raw bytes when it is not JSON; `inputs.catalog_state` is `null` when no catalog state is passed. `inputs.security_fingerprint` is the fingerprint of SEC-3.6.
- **PLN-6.4** **Context not admitted.** When the security context is rejected (E2-5 row 0a), `subject` and `inputs.security_fingerprint` are `null`, and the record is emitted with the error codes as its outcome.
- **PLN-6.5** **Authorization entries.** `authorization` has one entry per object E2 authorizes, at its first authorization, in the order E2 authorizes them (E2-4: each name as soon as it resolves). `object` is `dataset:<name>`, `field:<dataset>.<field>` or `metric:<name>`, with the names as the model declares them. `rule` is one of:
  - `access:<mode>`: a dataset decided by its `access` mode (`roles`, `authenticated` or `internal`); a dataset read as `authenticated` under DEV-4 is `access:authenticated`;
  - `no_access`: a dataset without `access`, denied (SCY-3.0);
  - `field_access:<roles>`: a field decided by its restriction (SCY-5.1.8); the roles are written sorted by code points, without duplicates, joined by `,`;
  - `no_field_restriction`: a field without restriction, granted;
  - `metric_components`: a metric decided by E2-4.3.
- **PLN-6.6** `row_policies` lists every row policy applied, written and ordered as in PLN-4.7. `decisions` holds the entries of the plan's `explanation` (PLN-7), in the same order; for a compile error, the entries recorded before the failure.
- **PLN-6.7** `compile_options` holds `dialect`. `load_options` holds every load option of architecture section 3.5 with the value the handle uses, defaults included; `model_name` is present only when it was passed, and `data_source` is the value written in every plan.
- **PLN-6.8** **Delegated actor.** The `actor` compile option is invocation metadata, authenticated by the host. It is written unchanged as `actor` in the audit record, and only there; the member is absent when the option is not passed. It is outside every hash: it changes neither the plan, the `plan_hash`, the cache key, the security fingerprint nor `inputs`. The compiler reads the actor only from the compile option, never from the security context, the query or the model.

## PLN-7 Explanation

Each entry of `explanation` records one decision:

| Field | Content |
|---|---|
| `code` | The rule ID that decides it (column "Code" of PLN-7.3). Stable across releases. |
| `step` | The stage that decides it, `E0` to `E6`. |
| `subject` | The object decided about, as `<kind>:<name>` (PLN-7.2). Stable across releases. |
| `decision` | A short English statement of the decision; for an E5-3 rewrite, exactly `applied` or `rejected`. |
| `reason` | An English statement of why, naming the condition that held or failed. Its wording may change between releases; hosts rely on `code`, `step`, `subject` and `decision`. |

- **PLN-7.1** Entries are ordered by `step` (`E0` first), then in emission order (PLN-7.3). An entry with the same `code` and `subject` as an earlier one is not written again.
- **PLN-7.2** `subject` kinds: `dataset:<name>`, `field:<dataset>.<field>`, `metric:<name>`, `relationship:<name>`, `policy:<dataset>.<policy>`, `summary:<name>`, `path:<from>-><to>`, `role:<role>`, `attribute:<name>` and `load_option:<name>`. Model names are written as the model declares them; roles and attribute names as the security context spells them. Values never appear (PLN-4.2).
- **PLN-7.3** These decisions, and only these, are recorded. Within a step, they are emitted in the order of this table, and each row in the order it states:

| Step | Code | Subject | Recorded when |
|---|---|---|---|
| `E0` | `DEV-4` | `load_option:grant_authenticated_to_all` | Always; `decision` states whether datasets without `access` are read as `authenticated`. |
| `E2` | `SEC-3.2` | `role:<role>` | For each role of the context that the model does not declare, in code-point order. |
| `E2` | `SEC-3.2a` | `attribute:<name>` | For each attribute of the context that the model does not declare, in code-point order. |
| `E2` | `QRY-4.2` | `field:<dataset>.<field>` of the `date_range` | A relative range is resolved; `reason` gives its unit, count and `include_current`, and names the parameter positions of its bounds, never the dates. |
| `E3` | `E3-3.3` | `path:<from>-><to>` | For each `paths` entry the query names and E3 accepts, in query order. |
| `E4` | `E4-4.0e` | `dataset:<home>` | A branch gets a `Deduplicate`, in branch order (E3-4.3). |
| `E4` | `E4-4.1` | `relationship:<name>` | A `left` join can produce a NULL group, in branch order, then path order. |
| `E4` | `E4-6.10` | `policy:<dataset>.<policy>` | A policy is propagated (E4-6.5), in the visit order of E4-6.9; `reason` names the datasets it reaches. |
| `E5` | `E5-3.1` to `E5-3.6` | As E5-3.8 states | A rewrite is applied, or considered and rejected (E5-3.8), in the order E5 runs the rewrites (E5-4 item 4), then branch order, then path order. |
| `E5` | `E5-4` | `summary:<name>` | A branch uses the summary; `decision` is `requires authorized executor` (E5-4 item 5, SUM-4.2). |

- **PLN-7.4** The explanation is deterministic (P3, PLN-2.4), and `compile` and `explain` give the same one (PLN-8.1). Each stage records its decisions as it makes them: an E4 entry stays when an E5 rewrite later changes the node it describes (E5-3.2 removing a `Deduplicate`, for example), and the rewrite has its own entry.

## PLN-8 Explain

- **PLN-8.1** `explain` returns exactly the document `compile` returns for the same inputs, with `contract` `menura.explanation`, `sql` `""` and `parameters` `[]`. `cache_key` is computed from the real plan, so it equals the one `compile` returns.
- **PLN-8.2** For inputs on which `compile` fails, `explain` returns the identical compile error.
- **PLN-8.3** `explain` ignores the `audit` and `actor` compile options and never emits an audit record.
