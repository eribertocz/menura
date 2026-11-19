# E0.h Build the internal model

## E0H-1 Properties

| ID | Property | Rule |
|---|---|---|
| E0H-1.1 | Immutable | Never changes after construction. A new model is a new value. |
| E0H-1.2 | Complete | Everything E1–E6 need is resolved: types, graph, components, interpreted extensions. No later stage reads the model document. |
| E0H-1.3 | Normalized | Expressions are stored as normalized trees. Formulas that differ only in whitespace, comments, keyword or function-name case, redundant parentheses, or the quoting of a name that needs none produce the same tree. Names are stored normalized (E0D-1.1, E0D-1.4); literals keep their value and type as written (`1.50` stays a decimal of scale 2); no algebraic rewriting (operand order, constant folding) is done. |
| E0H-1.4 | Ordered | Every collection is ordered by normalized name, so traversal is deterministic (P3). |
| E0H-1.5 | Located | Every element keeps its source location for diagnostics and explanations. |

## E0H-2 Content

| Element | Includes |
|---|---|
| Datasets | Source, typed fields with nullability, keys, access, field access, row policies, population, summary marker |
| Graph | Named edges, reachability, multiple-path pairs |
| Metrics | Normalized trees, components, classes, additivity, types |
| Security | Roles, attributes, parsed policies |
| Summaries | Mapping to their base and definition hash |
| Integrity checks | See E0H-4 |
| Unusable elements | Each with its load warning and the query error code of E0-4 |
| Warnings | Every warning emitted by E0 |
| Model hash | See E0H-3 |

## E0H-3 Hashes

- **E0H-3.1** A hash is the SHA-256 of the RFC 8785 canonical JSON of normalized trees, as 64 lowercase hex digits (CON-2).
- **E0H-3.2** The **model hash** covers the whole internal model except source locations, so reformatting a document without changing its meaning does not change the hash. It is the only model hash (MDL-3.1 refers to it).
- **E0H-3.3** The **summary definition hash** covers the normalized summary extension, the base dataset `source`, and the normalized tree and logical type of every field it refers to (dimension `base` fields and the fields read by each `base_aggregate`).
- **E0H-3.4** Hashes cover meaning only: never source locations, warnings or diagnostic messages. The JSON form of normalized trees is defined by the `model` crate. A release that changes any hash for the same input (model hash, summary definition hash or check `definition_hash`) says so in its release notes, because pipelines must then rebuild summaries and rerun checks.

## E0H-4 Integrity checks

E0 records the checks the model needs; `integrity_checks(handle, dialect)` generates them as the
[integrity-check contract](../../contracts/integrity-checks.md) specifies. Pipelines run them and
report the results in the catalog state.

| Check | Detects | Evidence enables |
|---|---|---|
| Key uniqueness of `to_columns` in `to` | Violated many-to-one cardinality | Skipping row-identity deduplication for that relationship |
| Orphan keys: `from_columns` values with no match in `to` | Missing referential integrity | Inner joins instead of left joins for that relationship |
| Summary integrity | A summary that differs from its base | Using the summary (CAT-7.2) |
| Population | Missing or unexpected entities per period | Semi-additive policies (population extension) |
| Semi-additive | A semi-additive metric whose snapshots break its declaration | Semi-additive policies (additivity extension) |

- **E0H-4.1** Without passing evidence for the current data versions, the compiler always takes the safe plan: left joins, deduplication by row identity, base relations. It never assumes referential integrity or cardinality.
- **E0H-4.2** Checks are logical plans. The `core` crate passes them to the same generator as E6, so a check and a query never translate an expression differently, and E0 never depends on a generator (S7).
- **E0H-4.3** E0 records one check per usable relationship for each of `key_uniqueness` and `orphan_keys`, one `summary` check per summary, one `population` check per population and one `semi_additive` check per semi-additive metric. Each has the `kind`, `target` (the normalized name), `check_id` and `definition_hash` of CHK-2 and CHK-3, sorted by `check_id`. Their SQL is generated later, as E0H-4.2 states.
