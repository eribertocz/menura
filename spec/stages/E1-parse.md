# E1 Parse the query

## E1-1 Purpose

Turn a query document, in either form, into one **query tree**. E1 checks the envelope and the
shape only. Every check that needs the model (names, filters, aliases, metrics) runs in E2, so
that error precedence is the same for both forms (E2-5).

## E1-2 Envelope

- **E1-2.1** The envelope is checked before JSON Schema validation, in this order: the document is JSON (`E_INVALID_QUERY`); `contract` and `contract_version` are supported (`EQRY-08`); exactly one of `query` and `sql` is present (`EQRY-01`); `time_zone` is a known IANA name (`EQRY-02`); every extension `version` is supported (`EQRY-06`). Then the document is validated against the query JSON Schema; any failure is `E_INVALID_QUERY`, located by JSON Pointer.
- **E1-2.2** Locations and members that cannot be read, as in CAT-9.1 and SEC-3.8. A document that is not valid UTF-8 JSON is `E_INVALID_QUERY` at the empty pointer, with line and column. A missing, non-string or unsupported `contract` or `contract_version` is `EQRY-08`, at `/contract` or `/contract_version`. `EQRY-01` is located at the empty pointer. `EQRY-02` applies only to a `time_zone` that is a string, located at `/time_zone`; a missing or non-string `time_zone` is left to the JSON Schema. `EQRY-06` applies only to an extension whose `data.version` is an integer other than 1, located at that `version`; an entry whose `version` cannot be read is left to the JSON Schema. Before 0.5.0, `E_NOT_YET_SUPPORTED` for the SQL form (VS-2) is located at `/sql`, and `E_EMPTY_AGGREGATION_QUERY` at `/query`.
- **E1-2.3** A known IANA name is the exact spelling, compared as a sequence of code points, of a zone or a link of the IANA time-zone database the compiler bundles. That database's version is the one `producer` declares (E6-5.5). `europe/madrid` is therefore `EQRY-02`, and `US/Eastern` is accepted.
- **E1-2.4** A query read through the former contract name reports `WQRY-01` at `/contract`; an extension read through the former namespace reports it at that extension's `data/namespace`. They are reported only for a query that passes every check of this section and its JSON Schema, in that order, after the security context's warning (SEC-3.9).

## E1-3 Wide-table form

- **E1-3.1** Each dimension and metric string is parsed as a reference: `dataset.field`, `dataset.metric` (development-line models) or `metric` (QRY-3.2). A reference is split into parts at each `.` outside a delimited identifier. Each part is a delimited identifier when it starts and ends with `"`, read as in E0D-1.4, and a regular identifier otherwise. E1 never rejects a reference. A reference that names nothing is `E_NAME_NOT_FOUND` in E2 (E2-3.2), located at the reference. This includes one with an empty part, a malformed delimited part or more than two parts.
- **E1-3.2** Each filter is parsed as an expression of the E0F-2 language. Whether it is boolean, reads one dataset, and contains no aggregate, metric reference or window function is checked in E2. A filter text the grammar rejects is `E0F-01`, and one with a construct outside the subset is `E0F-02` (E0F-9.1), located at the filter. These are checked right after the JSON Schema (E2-5 row 1): `filter` first, then each `filters` item in index order, and the first failure is reported.
- **E1-3.3** Extensions are parsed as defined in QRY-4.

## E1-4 SQL form

The SQL form follows Apache Ossie Layer 2 with its exact semantics. Version 1 implements the
subset below; anything else is `EQRY-05`, naming the construct. A construct is added only when
its Layer 2 semantics (row identity, measure input sets, no fan-out) are fully specified and
covered by the Layer 2 suite cases that test it.

| Clause | Version 1 subset |
|---|---|
| Statement | One `SELECT`. SQL comments are ignored and one trailing `;` is allowed. No `WITH`, set operators, subqueries, `DISTINCT` or window functions. |
| `FROM` | Exactly one relation: `<model>.<dataset> [[AS] alias]`, where `<model>` is the model name. No `JOIN`. References to the relation's fields and metrics MAY be unqualified or qualified by the alias or the dataset name. |
| `SELECT` items | A field of the relation; a scalar expression over fields of the relation (the E0F-2 language: operators, `CASE`, `CAST` (E0F-11) and the scalar functions of E0F-10); `AGG(metric)` or `MEASURE(metric)` of a metric exposed on the relation (QRY-5.2); a scalar expression over `AGG(…)` terms, literals and the same functions. An item that is not a plain field reference MUST have `AS alias` (`EQRY-13`). |
| `WHERE` | A boolean expression over fields of the relation. |
| `GROUP BY` | Exactly the non-aggregated `SELECT` items, by expression or position, or `GROUP BY ALL`. At least one `AGG(…)` item is required; without `GROUP BY`, every item MUST contain `AGG(…)` and the result is one row. |
| `ORDER BY` | Output names or positions, each with `ASC`/`DESC` and `NULLS FIRST`/`NULLS LAST`. |
| `LIMIT` | A positive integer literal, with a total order (QRY-4.6). |

- **E1-4.1** Semantics are those of Layer 2 section 8.1 for one relation: each `AGG(alias.metric)` evaluates the metric over the rows of the relation that pass `WHERE` and fall in the group. This coincides with a single-branch wide-table plan, so both forms lower to the same logical plan (P5).
- **E1-4.2** An arithmetic expression over `AGG(…)` terms is evaluated per group after the terms, with the default derived-metric rules (division by zero yields NULL, DEV-6).
- **E1-4.3** Result columns keep the `SELECT` order.
- **E1-4.4** `DATE_TRUNC('<grain>', x)` has the semantics of QRY-4.1: ISO weeks starting Monday, truncation in the query `time_zone`.
- **E1-4.5** A SQL text that does not parse is `EQRY-05`, located at `/sql`; the message gives the line and column of the failure within the SQL text, counted in code points.
- **E1-4.6** A query with no `AGG(…)` item is `EQRY-05`.
- **E1-4.7** A non-aggregated `SELECT` item that `GROUP BY` does not list, or a `GROUP BY` position outside the `SELECT` items, is `EQRY-05`.
- **E1-4.8** A `SELECT` item that mixes field references with `AGG(…)` terms is `EQRY-05`.
- **E1-4.9** These checks need the model, so they run in E2 at their rows of E2-5: a `FROM` relation whose `<model>` is not the model name is `E_NAME_NOT_FOUND` (row 3); a metric named without `AGG(…)` is `E_METRIC_IN_DIMENSIONS` (row 4); `AGG(…)` of a field is `E_FIELD_IN_METRICS` (row 5).
- **E1-4.10** An `ORDER BY` item without a direction is `ASC`; one without `NULLS FIRST` or `NULLS LAST` is `NULLS LAST`, in either direction.

## E1-5 Query tree

| Element | Content |
|---|---|
| Dimensions | A field reference with optional time grain, or (SQL form) a scalar expression over fields of one dataset; each with its output name |
| Metrics | Reference or inline expression over metrics, output name |
| Filters | Parsed expressions (E2 assigns each to its dataset) |
| Date range | Absolute bounds as written, or the relative range as written (resolved in E2, which raises `EQRY-03` without `reference_time`) |
| Time zone, reference time | `time_zone` (a known name, E1-2.3) and `reference_time` when present (an RFC 3339 instant); E2 uses them to resolve relative ranges (QRY-4.2), and E4 and E6 to truncate time grains (E6-5.2) |
| Paths | Relationships chosen by the query |
| Order, limit | If present |
| Column order | Wide-table form: dimensions, then metrics (Layer 3 section 3.6). SQL form: `SELECT` order. |
| Origin | Location of each element in the input |

- **E1-5.1** Output names are the alias as written, or else the last part of the reference as written: never normalized, and without the quotes of a delimited part (`orders."Net Amount"` gives `Net Amount`). Collisions are checked in E2.

## E1-6 Diagnostics

| Code | Condition |
|---|---|
| `E_INVALID_QUERY` | Not JSON, or fails the JSON Schema |
| `E_EMPTY_AGGREGATION_QUERY` | No dimensions and no metrics |
| `EQRY-01`, `EQRY-02`, `EQRY-05`, `EQRY-06`, `EQRY-08`, `EQRY-13` | As in QRY-7 |
| `E0F-01`, `E0F-02` | A filter text does not parse, or uses a construct outside the subset (E1-3.2) |
| `E_NOT_YET_SUPPORTED` | Before 0.5.0: the query uses the SQL form (VS-2) |
