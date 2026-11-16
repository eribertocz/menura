# Extension: additivity

**Key:** `additivity` · **Attached to:** Metric

## ADD-1 Purpose

State how a metric aggregates when its expression is not enough to know. Typical case: a debt
balance adds up across customers but not across time.

## ADD-2 Default and conflicts

- **ADD-2.1** Without this extension, behavior is deduced from the expression (E0G-3).
- **ADD-2.2** A declaration may only restrict the deduced behavior, never relax it. The schema guarantees it: there is no value that declares a metric additive.

## ADD-3 Schema

### ADD-3.1 `type: non_additive`

`{"additivity": {"type": "non_additive"}}` — the metric is always computed from base rows at
the query grain and never uses summaries. No other key is allowed.

### ADD-3.2 `type: semi_additive`

| Key | Required | Meaning |
|---|---|---|
| `over` | Yes | The one field across which the metric is **not** summed |
| `entity` | Yes | Fields that identify each snapshot entity (e.g. the customer) |
| `aggregation` | Yes | `last`, `first`, `min`, `max` or `avg` across `over` |
| `nulls` | Yes | `as_missing`, `keep`, `zero` or `error` |
| `snapshot_grain` | If `over` is a time field | `day`, `week`, `month`, `quarter` or `year` |
| `population` | If `over` is a time field | Name of the expected population (population extension) |
| `missing` | If `over` is a time field | `exclude`, `carry_forward`, `zero` or `error` for expected entities without a row |
| `unexpected` | If `over` is a time field | `include`, `exclude` or `error` for rows of entities the population did not expect |
| `selection` | If `aggregation` is `first` or `last` | `global_period` or `per_entity` |
| `max_carry_forward` | If `missing` is `carry_forward` | Positive integer, in `snapshot_grain` periods |
| `on_stale` | If `missing` is `carry_forward` | `exclude` or `error` when the last value is older than `max_carry_forward` |

Field references are written `dataset.field`.

## ADD-4 Meaning

| ID | Rule |
|---|---|
| ADD-4.1 | `selection: global_period` takes the last (or first) period with data inside the queried range, the same for every entity. `per_entity` takes each entity's own last (or first) row inside the range. |
| ADD-4.2 | An entity is **missing** in a period only if the population expects it in that period and it has no row. Completeness is never deduced from the history of rows. |
| ADD-4.3 | `missing`: `exclude` contributes nothing; `carry_forward` reuses the latest known value, even from before the range, if it is at most `max_carry_forward` periods old, else `on_stale` applies; `zero` contributes 0; `error` fails at execution. |
| ADD-4.4 | A row is **unexpected** if its entity is not expected in that period. `include` uses it, `exclude` ignores it, `error` fails at execution. |
| ADD-4.5 | `nulls`: `as_missing` treats the row as missing (only with a time `over`); `keep` keeps NULL; `zero` replaces it by 0; `error` fails at execution. |
| ADD-4.6 | After the rule across `over`, the metric is summed across every other dimension. |
| ADD-4.7 | **Evaluation unit.** The rule across `over` is applied separately for each result row. The "queried range" of a row is the intersection of the query `date_range` (or all periods with data, if none) with the row's own period of `over` when `over` (or a coarser grain of it) is a query dimension. So "2026 by category" takes the last month of 2026 for each category row, and "by year" takes the last month of each year. |
| ADD-4.8 | `global_period` means global within the row: the last (or first) period with data for any entity of that row. |
| ADD-4.9 | A query grain of `over` finer than `snapshot_grain` is an error (`E4-03`): a snapshot cannot be split. |
| ADD-4.10 | When the query does not group by `over`, the whole queried range is one evaluation unit. |

## ADD-5 Validation

1. `non_additive` admits no other key (`E0C-10`).
2. `semi_additive` requires its required keys (`E0C-11`).
3. Options that need time order require a time `over` (`E0C-12`).
4. `max_carry_forward` and `on_stale` require `missing: carry_forward`; `max_carry_forward` > 0 (`E0C-13`).
5. `selection` requires `first` or `last` (`E0C-14`).
6. `over` MUST NOT be part of `entity` (`E0C-15`).
7. `over` and `entity` fields exist and are reachable from the metric's single home dataset (`E0C-16`).
8. `population` exists, its `entity` matches in number, order and type, and a `periodic` population has `grain` = `snapshot_grain` (`E0C-17`).
9. A semi-additive metric never uses summaries in version 1.

## ADD-6 Integrity checks

For a time `over`, E0 generates checks (E0H-4): one row per entity and period at most;
no gaps between the first and last period with data; expected entities without a row;
rows of unexpected entities.

## ADD-7 Runtime codes

| Code | Condition |
|---|---|
| `RADD-01` | `missing: error` and an expected entity has no row |
| `RADD-02` | `unexpected: error` and an unexpected row exists |
| `RADD-03` | `nulls: error` and a NULL value exists |
| `RADD-04` | `on_stale: error` and a carried value is too old |

## ADD-8 Example

```json
{"namespace": "semantic_compiler", "version": 1,
 "additivity": {
   "type": "semi_additive", "over": "debt.period", "entity": ["debt.customer_id"],
   "aggregation": "last", "selection": "global_period", "snapshot_grain": "month",
   "population": "active_customers", "missing": "carry_forward", "max_carry_forward": 2,
   "on_stale": "error", "unexpected": "exclude", "nulls": "as_missing"}}
```

For 2026 by category: take the last month of 2026 with data; find the customers expected that
month; carry forward the latest balance of expected customers without a row if it is at most
two months old, else fail; ignore rows of unexpected customers; sum across customers per category.
