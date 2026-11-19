# E0.h Build the internal model

## E0H-1 Properties

| ID | Property | Rule |
|---|---|---|
| E0H-1.1 | Immutable | Never changes after construction. A new model is a new value. |
| E0H-1.2 | Complete | Everything E1–E6 need is resolved: types, graph, components, interpreted extensions. No later stage reads the model document. |
| E0H-1.3 | Normalized | Expressions are stored as normalized trees. Formulas that differ only in whitespace, comments, keyword or function-name case, redundant parentheses, or the quoting of a name that needs none produce the same tree. Names are stored normalized (E0D-1.1, E0D-1.4); literals keep their value and type as written (`1.50` stays a decimal of scale 2); no algebraic rewriting (operand order, constant folding) is done. |
| E0H-1.4 | Ordered | Every collection of named elements is ordered by normalized name (CON-5), so traversal is deterministic (P3). Fields are ordered within their dataset. Positional lists keep their order, because it carries meaning: key columns, `from_columns`/`to_columns` pairs, function arguments, `IN` lists, `CASE` branches, summary dimensions and measures. The unusable elements are ordered by kind (dataset, field, relationship, metric), then by qualified normalized name. |
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
| Descriptions | The `description`, `label` and `ai_context` of the semantic model, datasets, fields, relationships and metrics, as written (DSC needs them) |
| Unusable elements | Each with its load warning and every query error code of E0-4 and E0-4.2a |
| Warnings | Every warning emitted by E0 |
| Model hash | See E0H-3 |

- **E0H-2.1** The internal model holds only what E1–E6 and `describe` need. Expression variants other than `ANSI_SQL`, and `custom_extensions` of other vendors, are accepted (E0B-3.5) but are not part of it, so they never affect a hash.
- **E0H-2.2** Field types come from the admitted catalog schema, so a type change in the catalog schema changes the model hash.
- **E0H-2.3** The graph holds only usable relationships: it excludes relationships made unusable after E0.e (`E0F-05`, E0-4.2a). E0.h recomputes reachability and the multiple-path pairs over the remaining edges, with the E0.e algorithm. The extension reachability checks (`E0C-16`, `E0C-61`) stay in E0.e over the E0.e graph. A path that disappears this way is reported at query time (E3).

## E0H-3 Hashes

- **E0H-3.1** A hash is the SHA-256 of the RFC 8785 canonical JSON of normalized trees, as 64 lowercase hex digits (CON-2).
- **E0H-3.2** The **model hash** covers the whole internal model except source locations and the warnings (their codes and messages), but including each unusable element and its query error codes, so reformatting a document without changing its meaning does not change the hash. It is the only model hash (MDL-3.1 refers to it).
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
- **E0H-4.3** E0 records one check per usable relationship for each of `key_uniqueness` and `orphan_keys`, one `summary` check per summary, one `population` check per population of a usable dataset and one `semi_additive` check per usable semi-additive metric. Each has the `kind`, `target` (the normalized name: of the relationship, of the summary, of the population (unique in the model, POP-4.3), or of the metric; a development-line dataset metric is qualified as `<dataset>.<metric>`), `check_id` and `definition_hash` of CHK-2 and CHK-3, sorted by `check_id`. Their SQL is generated later, as E0H-4.2 states.
