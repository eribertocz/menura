# E0.e Analyze relationships

## E0E-1 Purpose

Build the relationship graph and expose, at load time, everything that could later produce an
ambiguous join or an inflated number (P6). E3 uses the graph to choose paths.

## E0E-2 Graph

- **E0E-2.1** Nodes: every dataset of the model except summaries (E5 handles those).
- **E0E-2.2** Edges: one per relationship, temporal relationships included (VAL-2.12), directed from `from` (many side) to `to` (one side), identified by its name.
- **E0E-2.3** Queries traverse edges only from many to one, as in Apache Ossie Layer 3.
- **E0E-2.4** A path is a sequence of edges with no repeated dataset, so cycles never produce infinite paths.

## E0E-3 Validation

| ID | Rule |
|---|---|
| E0E-3.1 | `from_columns` and `to_columns` have the same length (`E0E-01`, located at the relationship). |
| E0E-3.2 | The relationship direction declares many-to-one cardinality. If `to_columns` matches neither the `primary_key` nor a `unique_keys` entry of `to`, the model is valid but produces `W0E-02`, because the data could violate the declared cardinality. `W0E-02` is not reported for a temporal relationship (VAL-2.12). |
| E0E-3.3 | Correctness under violated cardinality: a home row counts once in each group it matches (E4-4 step 5), using a synthesized row identity (Apache Ossie Layer 2 section 3.2). Along a temporal relationship, a row matches the versions that cover its `from_time` (VAL-2.8, VAL-2.13). |
| E0E-3.4 | If `from_columns` also matches a key of `from`, the relationship is one-to-one. It is recorded, and in version 1 it is still traversed only from `from` to `to`. |
| E0E-3.5 | Self-relationships are allowed, never take part in a path, and produce `W0E-03`. Cycles are allowed; paths stay finite because they never repeat a dataset (E0E-2.4). |
| E0E-3.6 | A relationship whose `from` or `to` is a summary dataset is an error (`E0E-02`). |
| E0E-3.7 | Key types are checked in E0.f. |
| E0E-3.8 | A column list matches a key when both hold the same set of columns, compared as names normalized by E0D-1.1; order and repetition are ignored, because uniqueness does not depend on column order. |

## E0E-4 Multiple paths

- **E0E-4.1** More than one path between two datasets is allowed (for example two relationships to a calendar, or a diamond through two intermediate datasets).
- **E0E-4.2** E0 computes every pair of datasets with more than one path and emits `W0E-01` for each, located at the `from` dataset of the pair; warnings with the same `from` are ordered by their target dataset in document order.
- **E0E-4.3** The compiler never chooses between paths. A query that needs such a pair MUST name the path (query `paths` extension); otherwise E3 fails with `E_AMBIGUOUS_PATH`.

## E0E-5 Unconnected datasets

- **E0E-5.1** Datasets without relationships between them are allowed. A query combining metrics of different home datasets is valid only if every dimension is reachable from every metric's home dataset (checked in E3).

## E0E-6 Output

- The directed graph with named edges.
- For each dataset: the reachable datasets, and for each one whether there is exactly one path or more than one (paths are enumerated on demand in E3, with at most two needed to prove ambiguity).
- The pairs with multiple paths.

## E0E-7 Diagnostics

| Code | Condition |
|---|---|
| `E0E-01` | `from_columns` and `to_columns` differ in length |
| `E0E-02` | Relationship to or from a summary dataset |
| `W0E-01` | More than one path between two datasets |
| `W0E-02` | `to_columns` is not a declared key of `to` |
| `W0E-03` | Self-relationship; not used in paths |
| `W0E-04` | Invalid relationship; excluded from the graph (E0-4.1) |
