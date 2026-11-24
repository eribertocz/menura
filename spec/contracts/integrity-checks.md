# Contract: integrity checks

**Contract version:** 1.3.0 · **Owner:** `menura` · **Producer:** compiler (`integrity_checks`) · **Consumer:** data pipelines

## CHK-1 Purpose

Integrity checks turn assumptions into evidence. The compiler generates them; pipelines run them
after each load and report the results in the catalog state (`checks`, `aggregate_build.integrity_check`).

## CHK-2 Document

`{contract: "menura.integrity_checks", contract_version, model_hash, dialect, checks: [...]}`; each check:

| Field | Meaning |
|---|---|
| `check_id` | Stable identifier: `<kind>:<target>` |
| `kind` | `key_uniqueness`, `orphan_keys`, `summary`, `population`, `semi_additive` |
| `target` | Normalized name of the relationship, summary, population or metric (E0H-4.3) |
| `definition_hash` | See CHK-3 |
| `sql` | Read-only query in the dialect; generated like any plan (E6) |
| `parameters` | Positional parameters of `sql`, in position order; each has exactly one of `value` or `binding` (CHK-7) |
| `sources` | Catalog relations it reads, whose versions the result must report in `verified` |
| `build` | Only on a `summary` check, and required there: the summary's build query `{sql, parameters, columns}` (CHK-5.4) |

- **CHK-2.1** A check passes when its query returns **zero rows**. Returned rows are the violations, for diagnosis.
- **CHK-2.2** The check queries see all rows: no row policy applies to them. They are run by the pipeline's identity, never by end users.
- **CHK-2.3** A check whose query does not run to completion has no result; CAT-7.1.6 states how the catalog state reports it. Without a result there is no evidence, so the compiler takes the safe plan (CHK-4.1).
- **CHK-2.4** `sources` lists every catalog relation the query reads, each once, sorted by source in CON-5 order.
- **CHK-2.5** A check query returns every violation row: it has no `LIMIT` and no other bound on the number of rows it returns.
- **CHK-2.6** A result column that holds a field's value is named by the field's normalized name (E0H-1.1). The columns CHK-6 adds (`row_count`, `violation`, `base.<measure>`, `summary.<measure>`) follow the field columns; when such a name equals the name of a field column of the same result, it takes the suffix `_1`, or the smallest `_n` that makes it unique.

## CHK-3 Definition hash

For a `summary` check, `definition_hash` **is** the summary definition hash (E0H-3.3). This is
how a pipeline learns that hash: it copies it, unchanged, into the catalog state (CAT-7.1.3).
For every other kind, it is the SHA-256 (CON-2) of the canonical JSON of `{kind, target,
definition}`, where `definition` is the normalized form of what the check reads, and nothing else:

- `key_uniqueness`, `orphan_keys`: the relationship (`from`, `to` and the column pairs), the `source` of both datasets, and the normalized tree and logical type of every field named in the column pairs;
- `population`: the normalized population extension, the `source` of its dataset, and the tree and logical type of every field it reads;
- `semi_additive`: the normalized metric (tree and additivity extension), the `source` of its dataset, and the tree and logical type of every field it reads.

A change to any other element (another field of the same dataset, a description) does not change the hash.

- **CHK-3.1** A model change that affects a check changes its hash, so old evidence stops counting (CAT-7.3).

## CHK-4 Use of evidence and generation

Moved from E0H-4 at the 0.2.0 gate, because they are checked by the stages that use the checks (E4–E6, `integrity_checks`), not by E0.

- **CHK-4.1** Without passing evidence for the current data versions, the compiler always takes the safe plan: left joins, deduplication by row identity, base relations. It never assumes referential integrity or cardinality.
- **CHK-4.2** Checks are logical plans. The `core` crate passes them to the same generator as E6, so a check and a query never translate an expression differently, and E0 never depends on a generator (S7).

## CHK-5 Summary build and time zone

- **CHK-5.1** The compiler generates, for each summary, its **build query**: the read-only query that computes the summary's rows from its base for a range of base periods, from the same logical plan as the summary check (CHK-4.2). A pipeline builds a summary only with this query, so a summary and its check never differ in how they group, truncate or aggregate.
- **CHK-5.2** The build query and the summary check take the time zone as a bound parameter (the `time_zone` binding, CHK-7.3), used to truncate every time dimension of the summary as E6-5.2 states. The pipeline chooses it, binds the same value to both, and reports it in `aggregate_build.time_zone` (CAT-7.1.7). The time zone is not part of the summary definition hash (E0H-3.3).
- **CHK-5.3** A summary with a time dimension whose base field is a `timestamp` with time zone serves only queries in the time zone it was built in (CAT-7.2, condition 5). To serve several time zones, a model declares one summary per time zone.
- **CHK-5.4** Every `summary` check carries its summary's build query in `build`: `sql` (a read-only query in the dialect, generated as E6 states), `parameters` (as in CHK-2, with the same bindings as the check: CHK-7) and `columns`. No other kind carries `build`.
- **CHK-5.5** `build.columns` lists the summary's catalog columns in the summary's field order, which is the order of its fields by normalized name (E0H-1.4): for each field, the column its expression names. The build query returns exactly these columns, in this order, one row per group of the summary's dimensions: each dimension as its `base` field truncated to its `grain` (E6-5.2), each measure as its `base_aggregate` over the base rows of the group. Groups compare dimension values with `IS NOT DISTINCT FROM`, so NULL values form a group.
- **CHK-5.6** The build query reads only the base rows in the bound period range (CHK-7.4). The pipeline binds a range that covers whole periods at the coverage grain G (the grain of `coverage_field`): `period_from` is the first base period of a period of G, and `period_to` the last base period of a period of G. The same holds for the range bound to the summary check. So no summary row is built or checked from part of its period.

## CHK-6 Check queries

Each query returns one row per violation (CHK-2.1). Unless a rule says otherwise, it reads every row of the relations in `sources` (CHK-2.2).

- **CHK-6.1** **`key_uniqueness`.** The groups of the relationship's `to_columns` values in the `to` dataset where every value is non-NULL and `COUNT(*)` > 1. Columns: the `to_columns` fields in pair order, then `row_count`, the number of rows of the group as a 64-bit signed integer.
- **CHK-6.2** **`orphan_keys`.** The rows of the `from` dataset whose `from_columns` values are all non-NULL and that have no row of `to` with equal `to_columns` values, pair by pair (an anti-join). Columns: the `from_columns` fields in pair order. A `from` row with a NULL in any `from_columns` field is never a violation.
- **CHK-6.3** **`population` with `mode: validity`.** The union of: (a) the rows whose `valid_from` and `valid_to` are non-NULL and whose `valid_from` day is after their `valid_to` day, with `violation` `invalid_interval`; (b) each row that overlaps another row of the same entity, with `violation` `overlap`. Two rows overlap when both have non-NULL `entity` values that are equal field by field, both have a non-NULL `valid_from`, neither is an `invalid_interval` row, and each one's `valid_from` day is on or before the other's `valid_to` day (a NULL `valid_to` is open-ended). Days are those of POP-3.5. Two rows with equal values are two rows. Columns: the `entity` fields in declared order, `valid_from`, `valid_to`, then `violation`.
- **CHK-6.4** **`population` with `mode: periodic`.** The pairs of entity and period that more than one row has: rows with non-NULL `entity` values that are equal field by field and the same non-NULL `period` truncated to `grain` (POP-3.6). Columns: the `entity` fields in declared order, `period` holding the first day of the period as a `date`, then `row_count`, as in CHK-6.1.
- **CHK-6.5** **`semi_additive`.** The rows of the metric's home dataset (ADD-5.1.6) whose `over` and `entity` values, read as the metric reads them, are all non-NULL. The period of a row is its `over` value truncated to `snapshot_grain` (E6-5.2, POP-3.5). The query is the union of the ADD-6 conditions, each row with a `violation` text column naming its condition:
  - `duplicate_snapshot`: an entity and period that more than one row has;
  - `gap`: a period at `snapshot_grain` after the first and before the last period that has a row of any entity, that no row has; its entity columns are NULL;
  - `missing_entity`: an entity and period that the `population` expects (POP-3) for a period from the first to the last period that has a row, both inclusive, that no row has;
  - `unexpected_entity`: an entity and period that a row has and the `population` does not expect (ADD-4.4).

  Each entity and period appears at most once per condition. Columns: the `entity` fields in declared order, the `over` field holding the first day of the period as a `date`, then `violation`. When `over` is not a time field (ADD-5.1.5), only `duplicate_snapshot` applies, with the `over` value itself as the period and the `over` column holding that value.
- **CHK-6.5a** When an `over` or `entity` field of a semi-additive metric is not a field of its home dataset, `integrity_checks` emits no `semi_additive` check for that metric. This is not an error; the other checks of the model are unaffected.
- **CHK-6.6** **`summary`.** A full outer join of the base aggregate, recomputed from the base rows in the bound period range exactly as the build query computes it (CHK-5.5), and the summary rows whose `coverage_field` value lies in the same range of days (CHK-7.4), on every dimension with `IS NOT DISTINCT FROM`. It returns the rows where a side is missing or a measure differs: `violation` is `missing_in_summary` (a base group with no summary row), `missing_in_base` (a summary row with no base group) or `measure_differs` (both sides exist and at least one measure differs under `IS DISTINCT FROM`, with no cast or rounding). Columns: the dimension fields in the summary's field order, holding the value of whichever side exists; then, for each measure in field order, `base.<measure>` (the recomputed value) and `summary.<measure>` (the stored value); then `violation`.

## CHK-7 Parameters and bindings

- **CHK-7.1** A parameter has `position`, `logical_type` and exactly one of `value` or `binding`. A parameter with `value` is a value the compiler fixed, encoded as CON-4 requires; the pipeline binds it unchanged. A parameter with `binding` is a slot the pipeline fills at run time.
- **CHK-7.2** `binding` is `time_zone`, `period_from` or `period_to`. Each has `logical_type` `text`. A slot appears at most once in a query's `parameters`; its value is shared by every use in the SQL (E6-4.2). The pipeline binds the same values to a summary check and to the build query of the same build.
- **CHK-7.3** A query has a `time_zone` slot exactly when it computes a day or a period from a `timestamp` with time zone: a summary check or build query whose time dimension has such a `base` field (CHK-5.2); a `semi_additive` check whose `over`, or a time field of its `population`, is one; a `population` check whose `valid_from`, `valid_to` or `period` is one. The pipeline binds an IANA time-zone name; the day of such a value is its day in that time zone (E6-5.2).
- **CHK-7.4** A summary check and a build query have exactly one `period_from` slot and one `period_to` slot; no other kind has them. The pipeline binds periods in canonical form (CAT-5.1) at the grain of the base relation's `partitions`, `period_from` ≤ `period_to`. The query reads the base rows whose coverage day (the day of the `base` field of the coverage dimension, POP-3.5) is on or after the first day of `period_from` and on or before the last day of `period_to`: the range is inclusive at both ends, and a row whose coverage day is NULL is outside it. The query reads the grain of each bound from its shape (CAT-8, condition 6), so its SQL does not depend on the catalog state.

Schema: [`schemas/integrity-checks.schema.json`](schemas/integrity-checks.schema.json) · Example: [`examples/integrity-checks.example.json`](examples/integrity-checks.example.json)
