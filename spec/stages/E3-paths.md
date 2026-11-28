# E3 Resolve paths

## E3-1 Purpose

Choose, for every metric component and every filter, the join path to every dataset it needs,
following Apache Ossie Layer 3 (sections 4 and 5.3). The compiler never guesses a path.

## E3-2 Input and output

| | Content |
|---|---|
| Input | Bound query, relationship graph (E0E-6) |
| Output | Branches (E3-4), each with its join tree |

## E3-3 Paths and reachability

| ID | Rule |
|---|---|
| E3-3.1 | A step crosses a relationship from `from` to `to`, never against that many-to-one direction (E0E-2.3), and never a self-relationship (E0E-3.5). A path is a sequence of steps with no repeated dataset. The path from a dataset to itself is empty. A `paths` entry that breaks one of these rules is `EQRY-12` (QRY-4.3). A temporal relationship is a step like any other (VAL-2.12). |
| E3-3.1a | **Needed datasets.** A branch needs its dimension datasets and its filter datasets. The filter datasets are the datasets read by `filter` and by the `filters` items, and the dataset of the `date_range` dimension: it is needed, joined and checked for reachability as a filter dataset, and its errors are located at the `date_range` entry. A dataset that is only intermediate on a path is not needed. |
| E3-3.2 | If two or more paths lead from a home dataset to a needed dataset, the query MUST choose one with `paths` (QRY-4.3); otherwise `E_AMBIGUOUS_PATH`, even if one path is shorter. A `paths` entry for (`from`, `to`) governs only that pair: it chooses no path for another pair, even one whose paths pass through `to`. |
| E3-3.3 | A `paths` entry that chains correctly (QRY-4.3) but whose `from` is not the home dataset of any branch, or whose `to` is not a dataset any branch needs (E3-3.1a), is `E3-01`. So an entry whose `to` is only intermediate on a path is `E3-01`. A choice that no ambiguity requires is accepted and recorded in the explanation. |
| E3-3.4 | The datasets of a metric are its components' home datasets. A dataset is reachable from a metric only if it is reachable from every one of them. |
| E3-3.5 | The dataset of a `date_range` is a filter dataset (E3-3.1a), not a dimension dataset, unless a dimension reads it too. Every dimension dataset MUST be reachable from every metric: `E_NO_PATH` if no metric reaches it, `E_PARTIAL_DIMENSION_REACHABILITY` if only some do. |
| E3-3.6 | Every filter dataset (E3-3.1a) MUST be reachable from every metric (`E_UNSUPPORTED_FILTER_PATH`), located at its filter or at the `date_range` entry. A filter that reads no dataset needs no check. |
| E3-3.7 | Datasets needed only by row policies are resolved in E4, not here. |

## E3-4 Branches

- **E3-4.1** Components with the same home dataset form one **branch**; the components of a semi-additive metric form their own branch, stitched by the query keys (E4-5).
- **E3-4.2** A branch's **join tree** is the union of its paths to every dimension dataset and every filter dataset (E3-3.1a), the dataset of the `date_range` included.
- **E3-4.2a** The join tree holds one **join instance** per distinct path prefix. Paths that start with the same steps share the instances of those steps; the same dataset reached by two different step sequences is two instances (role-playing). Each query element is bound to the instance at the end of its own path.
- **E3-4.3** Branches are ordered by the normalized name of their home dataset (P3). Among branches with the same home dataset, the plain branch comes first, then each semi-additive branch, then each context branch, each kind ordered by the normalized name of its metric.

## E3-5 Dimension-only queries

- **E3-5.1** Without metrics, rows come from an **anchor**: a dimension dataset from which every other dimension dataset is reachable. No anchor is `E_NO_ANCHOR`; more than one is `E_AMBIGUOUS_PATH`. Both are located at `/query/dimensions`.
- **E3-5.2** The filter datasets (E3-3.1a) MUST be reachable from the anchor (`E_UNSUPPORTED_FILTER_PATH`, E2-5 row 16). With several anchors, each filter dataset MUST be reachable from every anchor; this is checked before the more-than-one-anchor error, which stays at row 17.
- **E3-5.3** The query becomes one branch with the anchor as home dataset and no components.

## E3-6 Diagnostics

| Code | Condition |
|---|---|
| `E_AMBIGUOUS_PATH` | More than one path and none chosen, or more than one anchor |
| `E_NO_PATH` | A dimension no metric reaches |
| `E_PARTIAL_DIMENSION_REACHABILITY` | A dimension only some metrics reach |
| `E_UNSUPPORTED_FILTER_PATH` | A filter or `date_range` dataset some metric (or an anchor) cannot reach |
| `E_NO_ANCHOR` | Dimension-only query without an anchor |
| `E3-01` | A `paths` entry that no branch uses |
