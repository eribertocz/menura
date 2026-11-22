# Extension: context modifiers

**Key:** `context` · **Attached to:** Metric

## CTX-1 Purpose

Evaluate a metric in a context different from the query's: period to date, a prior period, or
a share of a total. The modified metric is declared in the model, so every consumer gets the
same definition, and it compiles to standard SQL.

## CTX-2 Schema

```json
{"context": {"base": "<metric>", "modifier": {"type": "<type>", ...}}}
```

- **CTX-2.1** `base` names the metric evaluated in the modified context. The modified metric's Apache Ossie `expression` MUST be `AGG(<base>)`.

| `type` | Keys | Value for each result row |
|---|---|---|
| `period_to_date` | `time_dimension`, `grain` | `base` from the start of the `grain` period containing the row's time value up to the end of the row's period |
| `prior_period` | `time_dimension`, `grain`, `offset` | `base` for the row's period shifted back `offset` units of `grain` |
| `share_of_total` | `remove` (list of fields, or `"all"`) | `base` at the row's grain divided by `base` with the `remove` dimensions taken out of the grouping |

## CTX-3 Rules

| ID | Rule |
|---|---|
| CTX-3.1 | `period_to_date` and `prior_period` require `time_dimension` in the query, at a grain equal to or finer than `grain`, and a time grain when it is a `timestamp` (`E4-02`, E4-8.6). |
| CTX-3.2 | `prior_period` shifts the time key and the query's date range on `time_dimension` only; every other filter is unchanged. A shifted period outside the data gives the base metric's empty-input value. |
| CTX-3.3 | `share_of_total` removes dimensions from the grouping only. Query filters remain, and row policies always remain (E4-6.8). |
| CTX-3.4 | `share_of_total` divides with NULL on a zero total. |
| CTX-3.5 | Each modified metric is planned as its own branch, stitched to the query by the query keys (E4-8). |
| CTX-3.6 | A modified metric is recomposable from a summary only if its base is and the summary covers the modified range. |
| CTX-3.7 | Derived metrics combine modified metrics: `AGG(sales) - AGG(sales_prior_year)` is the year-over-year change. |

## CTX-3a Definitions

Let `g` be the query grain of `time_dimension` and `t` a result row's time value (a period of grain `g`).

| ID | Definition |
|---|---|
| CTX-3a.1 | `period_to_date`: the base metric over the interval from the start of the `grain` period containing `t` to the end of `t`, with every query filter applied except the query `date_range` on `time_dimension`, which only selects the result rows. So year-to-date for January to March starts on 1 January even if the date range starts in March. `g` MUST be equal to or finer than `grain` (`E4-02`). |
| CTX-3a.2 | `prior_period`: the base metric over the period `t` shifted back by `offset` units of `grain`. `g` MUST be equal to or finer than `grain` (`E4-02`). A shift by months or quarters maps a day to the same day number, or to the last day of the target month when that day does not exist (31 March − 1 month = 28 or 29 February). A shift by years maps 29 February to 28 February. Weeks shift by 7 days per unit. A week row shifted by months or quarters is the ISO week that contains its Monday shifted as a day. A week row shifted by years is the ISO week with the same number in the shifted ISO week-numbering year (week 53 maps to week 52 when the target year has no week 53). |
| CTX-3a.3 | `share_of_total`: the base metric at the row's grain divided by the base metric at the row's grain with the `remove` dimensions taken out. The result type is a 64-bit float. |
| CTX-3a.4 | Without a `date_range`, the result rows are those of the base metric without the modifier; each row's modified value is computed by CTX-3a.1–3a.2 from all data. |
| CTX-3a.5 | A `remove` field that is not a query dimension has no effect. |

## CTX-4 Examples

```json
{"context": {"base": "billed", "modifier": {"type": "period_to_date", "time_dimension": "invoices.invoice_date", "grain": "year"}}}
{"context": {"base": "billed", "modifier": {"type": "prior_period", "time_dimension": "invoices.invoice_date", "grain": "year", "offset": 1}}}
{"context": {"base": "billed", "modifier": {"type": "share_of_total", "remove": ["customers.region"]}}}
```

## CTX-5 Validation

| Code | Condition |
|---|---|
| `E0C-60` | `base` does not exist, or the expression is not `AGG(<base>)` |
| `E0C-61` | `time_dimension` is not a time field reachable from the base metric |
| `E0C-62` | `offset` is not a positive integer |
| `E0C-63` | A `remove` field does not exist |
| `E0C-64` | `base` is itself a modified metric (version 1 allows one level) |
| `E0C-65` | A `period_to_date` base is semi-additive, or reads a semi-additive metric |
| `E4-02` | The query lacks the required time dimension, or its grain is coarser than `grain` |

### CTX-5.1 Validation details

| ID | Rule |
|---|---|
| CTX-5.1.1 | **Shape, in E0.c.** `context` is an object with exactly `base` (a string) and `modifier` (an object). `modifier.type` is one of the three types of CTX-2, and `modifier` has exactly the keys of its type. `grain` is `day`, `week`, `month`, `quarter` or `year`. `time_dimension` and each `remove` item are strings. `remove` is a non-empty list without repeats, or the string `all`. A missing key, a wrong type or a value outside its set is `E0C-03`. A key that the declared type does not list is `E0C-02`; when `modifier.type` is missing or invalid, only keys that no type defines are `E0C-02` (E0C-2.4). A non-positive or non-integer `offset` is `E0C-62`. `context` is allowed on model metrics only; on a dataset metric of the development format it is `E0C-05`. A context metric with `additivity` is `E0C-36` (DER-4.4). |
| CTX-5.1.2 | **Field references.** `time_dimension` and each `remove` item are written `dataset.field` and read as ADD-5.1.3 states; any other form is `E0C-03`. |
| CTX-5.1.3 | **Base, in E0.d.** `base` names a model metric, compared under E0D-1.1 and E0D-1.3. The metric's expression is exactly one `AGG(…)` term that names that metric under the same comparison. Otherwise, and for a metric that has both `context` and `derived`, `E0C-60`. A base that is itself a context metric is `E0C-64`; a derived base is allowed (CTX-3.7). A `period_to_date` metric whose base declares `additivity` with `type: semi_additive`, or reads such a metric through derived metrics, is `E0C-65`: a running total of snapshots has no meaning. A `remove` field whose dataset or field does not exist is `E0C-63`. |
| CTX-5.1.4 | **Time dimension (`E0C-61`).** In E0.e, the dataset and field of `time_dimension` must exist, and the dataset must be each home dataset of the base, or be reached from each by a path (E0E-2.3, E0E-2.4). A base made unusable before E0.e is not checked; a base with no home dataset gets only the existence check. In E0.f, the field must have kind `date` or `timestamp`; this is checked only when E0.f typed it. |
| CTX-5.1.5 | **Graph.** Each context metric adds to the metric graph of DER-5.1.4 one edge to its base, so a cycle through derived and context metrics is `E0C-34`. |
| CTX-5.1.6 | **Unusable base.** A context metric whose base is unusable is unusable too, with `W0G-01` at the metric; its query error code is that of the base. |
| CTX-5.1.7 | **Type and classification.** The result type is the base's type, except for `share_of_total`, which is a 64-bit float (CTX-3a.3). The result is always nullable. A context metric is not a base metric, so E0G-2 does not apply. Its home datasets are those of its base, it is recomposable only if its base is (CTX-3.6), and its recorded additivity is non-additive. |
