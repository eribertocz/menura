# Extension: summaries

**Key:** `aggregate` · **Attached to:** Dataset

## SUM-1 Principle

A summary is a precomputed aggregate of a base dataset. E5 uses it instead of the base only if
the result is **exactly** the same. A summary never changes a number and never widens what a
user can see. When in doubt, the base is used.

## SUM-2 Declaration

A summary is a normal Apache Ossie dataset (its `source` and `fields` are validated like any
other) with `access: {"mode": "internal"}` and:

| Key | Required | Meaning |
|---|---|---|
| `of` | Yes | Base dataset; never another summary |
| `dimensions` | Yes (min. 1) | `{field, base, grain?}`: summary field, base field, and grain if the base field is a time field |
| `measures` | Yes (min. 1) | `{field, base_aggregate}`: summary field and the aggregate that produced it, e.g. `SUM(invoices.amount)` |
| `coverage_field` | Yes | Time field of the summary whose coverage the catalog reports |

- **SUM-2.1** `base_aggregate` is a single `SUM`, `COUNT(x)`, `MIN` or `MAX` call over base fields, optionally with `FILTER (WHERE …)` or a `CASE` argument.

## SUM-3 Conditions for use

| ID | Condition |
|---|---|
| SUM-3.1 | **Equivalence.** Every component of the metric matches a summary measure after normalization, and recomposes exactly, preserving its empty-input value: `SUM(x)`→`SUM(s)`; `COUNT(x)`→`COALESCE(SUM(c), 0)`; `MIN`→`MIN`; `MAX`→`MAX`; `AVG(x)`→`SUM(s) / NULLIF(COALESCE(SUM(c), 0), 0)`, where `s` is `SUM(x)` and `c` is `COUNT(x)` of the same argument and filter. Holistic aggregates never use summaries. |
| SUM-3.2 | **Exclusions.** Non-additive and semi-additive metrics, branches that need deduplication, and components whose argument is a float (float addition is not associative, so a recomposed sum can differ from the base) never use summaries in version 1. |
| SUM-3.3 | **Grain.** Every query dimension and filter field is a summary dimension with the same `base`, or is reached from the base through a relationship whose `from_columns` the summary keeps. |
| SUM-3.4 | **Time grain.** The query grain derives from the summary grain by exact grouping: `day` → all; `week` → `week`; `month` → `month`, `quarter`, `year`; `quarter` → `quarter`, `year`; `year` → `year`. |
| SUM-3.5 | **Time filter alignment.** A date range on the coverage dimension starts and ends on boundaries of the summary grain. |
| SUM-3.6 | **Policy preservation.** The user has access to the base; every applicable policy of the base can be applied to summary columns with the same `base`; every propagated policy has its relationship columns kept. A summary declares no policies of its own. |
| SUM-3.7 | **Freshness.** All conditions of CAT-7.2, including the time zone. |

## SUM-4 Visibility

- **SUM-4.1** A summary is never named by a query and never offered as a dimension.
- **SUM-4.2** Its physical relation MUST be readable only by the authorized executor's identity. The compiler cannot enforce this (P2); it is a deployment requirement, and plans that use a summary say so in the explanation.

## SUM-5 Integrity check

For each summary, E0 generates a query that recomputes it from the base within its coverage and
compares row by row (E0H-4).

## SUM-6 Validation

| Code | Condition |
|---|---|
| `E0C-50` | `of` missing or is a summary |
| `E0C-51` | A `field` or `base` does not exist |
| `E0C-52` | Invalid `base_aggregate` |
| `E0C-53` | Time dimension without `grain`, or `grain` on a non-time field |
| `E0C-54` | `coverage_field` is not a time dimension of the summary |
| `E0C-55` | Summary declares `row_access` |
| `E0C-56` | Summary `access` is not `internal` |
