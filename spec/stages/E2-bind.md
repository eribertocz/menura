# E2 Bind and authorize

## E2-1 Purpose

Resolve every name in the query tree against the model, and authorize every object the query
uses against the security context.

## E2-2 Input and output

| | Content |
|---|---|
| Input | Query tree, internal model, security context |
| Output | Bound query: every reference points to a model element; every element is authorized |

## E2-3 Resolution

| ID | Rule |
|---|---|
| E2-3.1 | Names compare under the identifier rules of E0D-1. |
| E2-3.2 | Every reference MUST resolve (`E_NAME_NOT_FOUND`). |
| E2-3.3 | A dimension MUST name a field (`E_METRIC_IN_DIMENSIONS`), except an expression dimension of the SQL form, which MUST read fields of one dataset and no metric; a metric MUST name a metric (`E_FIELD_IN_METRICS`). E4 groups by the dimension's expression. |
| E2-3.4 | A time grain or date range MUST apply to a field with `is_time: true` and logical kind `date` or `timestamp` (`E2-01`). |
| E2-3.5 | Every relationship named in `paths` MUST exist (`E_NAME_NOT_FOUND`). |
| E2-3.6 | The type of every filter is checked as in E0.f, with the same rules and codes: a type error inside a filter (`E0F-03`–`E0F-08`) is reported with its E0.f code, and a filter whose type is not boolean is `E0F-06`. Each is located at the filter. |

## E2-4 Authorization

Each name is authorized as soon as it resolves, before any further check on it, so that an
unauthorized user learns nothing from later diagnostics.

| ID | Rule |
|---|---|
| E2-4.1 | A query **uses** a dataset when it takes from it a dimension, a metric component, a filter, a field named in any query extension (`time_grains`, `date_range`, `order_by`), a population or carry-forward read of a metric, or when it is reached by a path the query names. Every used dataset MUST be authorized by its `access` rule. Datasets on paths that E3 chooses are authorized by construction (E2-4.8). |
| E2-4.2 | Every field the query names anywhere, and every field a join path reads, MUST be authorized by its dataset's field restrictions (SCY-3.1, SCY-3.1a). |
| E2-4.3 | Every metric MUST be authorized: every dataset it reads is authorized, and, when the metric is restricted (SCY-5.1.8), the user holds one of the roles of that restriction. Because SCY-5.1.8 takes the intersection of the restrictions of the fields the metric reads, this can deny a metric even when each of those fields is authorized on its own (deny by default, P9). |
| E2-4.4 | An unauthorized dataset, field or metric is reported exactly as a nonexistent one: `E_NAME_NOT_FOUND`, with the same message (PLN-5.2). |
| E2-4.5 | Datasets that E4 joins only to apply a row policy are not "used" and need no authorization; their columns never appear in the output. |
| E2-4.6 | Summary datasets (`access: internal`) can never be named in a query. |
| E2-4.7 | Every authorization decision is recorded for the audit record (PLN-6). |
| E2-4.8 | E2 computes the **authorized subgraph**: the datasets whose `access` admits the user, and the relationships whose two ends are both such datasets (an *authorized relationship*). E3 resolves paths in that subgraph only; a named path that uses any other relationship is `E_NAME_NOT_FOUND`. Joins needed only to apply row policies are added in E4, outside this subgraph, and are never named in diagnostics. |
| E2-4.9 | Row policies apply to every read of a dataset, including population and carry-forward reads. Runtime failures of additivity policies count only rows the user may see. |

## E2-5 Error precedence

When a query meets the conditions of several errors, the first in this table is raised. Rows
1–17 are Apache Ossie Layer 3 section 8, in its order; the compiler's own codes are placed where
their check runs.

| # | Code | Stage |
|---|---|---|
| 0a | Security context, in the order of SEC-3.8: not JSON (`ESEC-01`), `ESEC-03` (unsupported contract or version), then `ESEC-01` (schema, then declared type or encoding) | `compile` entry, before E1 |
| 0b | Envelope, in the order of E1-2.1: not JSON (`E_INVALID_QUERY`), `EQRY-08`, `EQRY-01`, `EQRY-02`, `EQRY-06` | E1 |
| 1 | `E_INVALID_QUERY` (JSON Schema), then `E0F-01` and `E0F-02` (filter text, E1-3.2), then `EQRY-05` and `EQRY-13` (SQL form) | E1 |
| 2 | `E_EMPTY_AGGREGATION_QUERY` | E1 |
| 3 | `E_NAME_NOT_FOUND` (unknown or unauthorized) | E2 |
| 4 | `E_METRIC_IN_DIMENSIONS` | E2 |
| 5 | `E_FIELD_IN_METRICS` | E2 |
| 6 | `E_AGGREGATE_IN_WHERE` | E2 |
| 7 | `E_WINDOW_IN_WHERE` | E2 |
| 8 | `E_CROSS_DATASET_FILTER` | E2 |
| 9 | `E_NAME_COLLISION` | E2 |
| 10 | `E_UNAGGREGATED_FINER_GRAIN_REFERENCE` | E2 (metric recorded unusable in E0.g) |
| 11 | `E_DEFERRED_WINDOWED_METRIC` | E2 |
| 12 | `E_AMBIGUOUS_MEASURE_GRAIN` | E2 |
| 12a | Any element marked unusable in E0 (E0-4 table): its query error code, in this order: `E0B-08`, `E0B-09`, `E0D-03`–`E0D-07`, `E0F-*`, `E0G-01`, `E0G-02`, `E0C-37` | E2 |
| 12b | Filter typing (`E0F-03`–`E0F-08`, E2-3.6), `E2-01`, `EQRY-09`, `EQRY-10`, `EQRY-11`, `EQRY-12`, `EQRY-03`, then `E_INVALID_QUERY` for a relative range outside the years 0001–9999, then `EQRY-04`, in that order | E2 |
| 12c | `E_NOT_YET_SUPPORTED`, before 1.0.0 only (VS-2; the SQL form is detected at row 1) | E2 |
| 13 | `E_NO_PATH` | E3 |
| 14 | `E_PARTIAL_DIMENSION_REACHABILITY` | E3 |
| 15 | `E_NO_ANCHOR` | E3 |
| 16 | `E_UNSUPPORTED_FILTER_PATH` | E3 |
| 17 | `E_AMBIGUOUS_PATH`, then `E3-01` | E3 |
| 18 | `ESEC-02` | E4 |
| 19 | `E4-02`, `E4-03` | E4 |

- **E2-5.1** E1 parses only; every check that needs names (rows 3–12b) runs in E2, so the order above holds whatever the form.
- **E2-5.2** Only the first error is guaranteed; later errors of the same stage MAY follow it in the document, in query order. Conformance cases compare the first code.
- **E2-5.3** **Query order** is fixed by the query tree, never by the key order of the JSON document. Wide-table form: the dimensions, the metrics, `filter`, the `filters` items, the `time_grains` items, `date_range`, the `order_by` items, the `paths` entries and `limit`, each list in index order. Within an element, its references are taken in the order they appear: in the text of an expression, or in the members of an extension entry (`from`, `to`, then `relationships` in order). SQL form: the relation of `FROM`, then the clauses in the order of the E1-4 table. Each row of the table above is checked over every element before the next row, and within a row the first element in query order that fails is reported, located at that element.
- **E2-5.4** Row 12c reports the first element in query order that needs a capability outside the version's slice (VS-1), located at that element. When that element needs several such capabilities, the message names the one whose version comes first in table 7.1, and that version.
- **E2-5.4a** Two capabilities of VS-1 belong to the whole query. **Several datasets:** the home dataset is the dataset of the first element in query order that reads one, and the error is located at the first element that reads another dataset; a metric whose own components span several datasets is located at that metric. **Propagation:** located at the first element that reads the home dataset. When one element needs several capabilities of the same version, the message names the first in this order: several datasets, `paths`, derived metric, row-policy propagation (0.4.0); additivity, population (0.5.0); context metric (0.8.0).
- **E2-5.5** Output names are compared after the upper-case mapping of E0D-1.1, whether they come from an alias, a regular part or a delimited part, because DuckDB compares column names without regard to case. `Region` and `region` collide (`E_NAME_COLLISION`), and an `order_by` `output` matches an output name the same way (`EQRY-10` when none matches). The result keeps each name as written (E1-5.1).
- **E2-5.6** At row 12b, `EQRY-03` is reported for a filter or relative `date_range` that needs "now", and also for a metric or dimension whose model expression uses `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP`, directly or through the fields and metrics it reads (QRY-2), located at that metric or dimension. Row policies are checked in E4 (E4-6.9b).

## E2-6 Diagnostics

| Code | Condition |
|---|---|
| `E_NAME_NOT_FOUND` | Unknown or unauthorized name |
| `E_METRIC_IN_DIMENSIONS` | A metric used as a dimension |
| `E_FIELD_IN_METRICS` | A field used as a metric |
| `E2-01` | Time grain or date range on a field that is not a time dimension |
| `E_UNAGGREGATED_FINER_GRAIN_REFERENCE`, `E_DEFERRED_WINDOWED_METRIC`, `E_AMBIGUOUS_MEASURE_GRAIN` | The query names a metric made unusable by that rule (E0G-2) |
| `EQRY-03`, `EQRY-04`, `EQRY-09`–`EQRY-12` | As in QRY-7 |
| `E_AGGREGATE_IN_WHERE` | Aggregate or metric in a filter |
| `E_WINDOW_IN_WHERE` | Window function in a filter |
| `E_CROSS_DATASET_FILTER` | A filter reads fields of more than one dataset |
| `E_NAME_COLLISION` | Duplicate output name |
| `E_NOT_YET_SUPPORTED` | Before 1.0.0: the query needs a capability outside the version's slice (VS-1–VS-3) |
