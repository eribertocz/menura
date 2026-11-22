# Contract: integrity checks

**Contract version:** 1.1.0 · **Owner:** `menura` · **Producer:** compiler (`integrity_checks`) · **Consumer:** data pipelines

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
| `sql`, `parameters` | Read-only query in the dialect; generated like any plan (E6) |
| `sources` | Catalog relations it reads, whose versions the result must report in `verified` |

- **CHK-2.1** A check passes when its query returns **zero rows**. Returned rows are the violations, for diagnosis.
- **CHK-2.2** The check queries see all rows: no row policy applies to them. They are run by the pipeline's identity, never by end users.
- **CHK-2.3** A check whose query does not run to completion has no result; CAT-7.1.6 states how the catalog state reports it. Without a result there is no evidence, so the compiler takes the safe plan (CHK-4.1).

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
- **CHK-5.2** The build query and the summary check take the time zone as a bound parameter, used to truncate every time dimension of the summary as E6-5.2 states. The pipeline chooses it, binds the same value to both, and reports it in `aggregate_build.time_zone` (CAT-7.1.7). The time zone is not part of the summary definition hash (E0H-3.3).
- **CHK-5.3** A summary with a time dimension whose base field is a `timestamp` with time zone serves only queries in the time zone it was built in (CAT-7.2, condition 5). To serve several time zones, a model declares one summary per time zone.
- **CHK-5.4** The build query and its parameters are part of the integrity-check document from version 0.7.0, with the `integrity_checks` output (VS-5); its fields are added to this contract then, as a minor version.

Schema: [`schemas/integrity-checks.schema.json`](schemas/integrity-checks.schema.json) · Example: [`examples/integrity-checks.example.json`](examples/integrity-checks.example.json)
