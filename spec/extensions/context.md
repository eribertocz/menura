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
| CTX-3.1 | `period_to_date` and `prior_period` require `time_dimension` in the query, at a grain equal to or finer than `grain` (`E4-02`). |
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
| CTX-3a.2 | `prior_period`: the base metric over the period `t` shifted back by `offset` units of `grain`. `g` MUST be equal to or finer than `grain` (`E4-02`). A shift by months or quarters maps a day to the same day number, or to the last day of the target month when that day does not exist (31 March − 1 month = 28 or 29 February). A shift by years maps 29 February to 28 February. Weeks shift by 7 days per unit. A week row shifted by months, quarters or years is the ISO week with the same number in the shifted ISO week-numbering year (week 53 maps to week 52 when the target year has no week 53). |
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
| `E4-02` | The query lacks the required time dimension, or its grain is coarser than `grain` |
