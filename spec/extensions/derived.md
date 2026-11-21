# Extension: derived metrics

**Key:** `derived` · **Attached to:** Metric

## DER-1 Purpose

Define a metric computed from other metrics, such as `collection_rate = collected / billed`.

## DER-2 Formula

- **DER-2.1** The formula is the `ANSI_SQL` variant of the metric's Apache Ossie `expression`. It refers to other metrics with `AGG(metric_name)`. This follows the development-line `AGG()` form, extended to model metrics as declared deviation DEV-5.
- **DER-2.2** The `derived` key marks the metric as derived. `AGG(metric)` of a model metric, in a metric marked neither `derived` nor `context`, makes that metric unusable (`E0C-37`, E0-4.1). `AGG(dataset.metric)` in a development-line model is valid Apache Ossie and is compiled as such.
- **DER-2.3** Allowed: scalar expressions over `AGG(…)` terms and literals: arithmetic, `CASE`, `COALESCE`, `NULLIF`, scalar functions. Forbidden: aggregate functions and field references.
- **DER-2.4** A derived metric may use other derived metrics, without cycles.

```yaml
metrics:
  - name: collection_rate
    expression:
      dialects:
        - dialect: ANSI_SQL
          expression: AGG(collected) / AGG(billed)
    custom_extensions:
      - vendor_name: COMMON
        data: '{"namespace": "menura", "version": 1, "derived": {"on_division_by_zero": "null"}}'
```

## DER-3 Schema

| ID | Key | Required | Meaning |
|---|---|---|---|
| DER-3.1 | `on_division_by_zero` | If the formula divides | `null` (that division yields NULL) or `error` (execution fails with `RDER-01`) |

## DER-4 Meaning

| ID | Rule |
|---|---|
| DER-4.1 | Inputs first, formula after: each input is computed at the query grain with all its rules (additivity, filters, row policies); then the formula is applied row by row. A rate is the quotient of sums, never the sum of quotients. |
| DER-4.2 | A derived value is never re-aggregated. At another grain, the inputs are recomputed and the formula applied again. |
| DER-4.3 | Inputs may come from different home datasets; each is a separate branch (E4-5). |
| DER-4.4 | A derived metric has no `additivity` of its own; it is recomposable only if every input is. |

## DER-5 Validation

| Code | Condition |
|---|---|
| `E0C-30` | Aggregate function in the formula |
| `E0C-31` | Field reference in the formula |
| `E0C-32` | `AGG(…)` names a metric that does not exist |
| `E0C-33` | The formula refers to no metric |
| `E0C-34` | Dependency cycle; the message lists the full cycle (`a → b → a`) |
| `E0C-35` | The formula divides and `on_division_by_zero` is missing |
| `E0C-36` | Derived metric with an `additivity` extension |
| `E0C-37` | `AGG(…)` in a metric marked neither `derived` nor `context` |

### DER-5.1 Validation details

| ID | Rule |
|---|---|
| DER-5.1.1 | **Shape, in E0.c.** `derived` is an object. Its only key is `on_division_by_zero`, whose value is the string `null` or the string `error`. A wrong type or value is `E0C-03`, and an unknown key is `E0C-02`. `derived` is allowed on model metrics only; on a dataset metric of the development format it is `E0C-05`. A derived metric with `additivity` is `E0C-36`. |
| DER-5.1.2 | **Inputs, in E0.d.** `AGG(name)` names a model metric, and `AGG(dataset.metric)` (development format) names a dataset metric. Both are compared under E0D-1.1 and E0D-1.3. A name with no such metric is `E0C-32`. A formula with no `AGG(…)` is `E0C-33`. `AGG(…)` in a model metric that is neither `derived` nor `context` is `E0C-37`, except `AGG(dataset.metric)` in the development format (DER-2.2), which is read as a derived metric without `on_division_by_zero` (division follows E0F-6.1). `AGG(…)` in a field or a dataset metric is `E0F-02`. |
| DER-5.1.3 | **Formula, in E0.f.** An aggregate function anywhere in the formula is `E0C-30`; a field reference is `E0C-31`. The formula "divides" when it contains the operator `/` or `%`. `AGG(m)` has the result type and nullability of `m`, and the formula is typed by E0F-5a. |
| DER-5.1.4 | **Graph and cycles, in E0.f (`E0C-34`).** Nodes are the metrics, and edges go from a derived metric to each metric it names in `AGG(…)` (E0G-5.1). The context extension adds its own edges. Cycles are found by a depth-first search that starts from each metric in document order and follows the `AGG(…)` terms in the order they are written. Each cycle is reported once, at the metric of the cycle that comes first in document order, and the message lists the cycle starting there (`a → b → a`; `a → a` for a metric that names itself). |
| DER-5.1.5 | **Unusable inputs.** A derived metric with an input that is unusable is unusable too, with `W0G-01` at the metric; its query error code is that of its first unusable input in the order written. |
| DER-5.1.6 | **Classification (E0G-5).** A derived metric is not a base metric: E0G-2 does not apply to it. Its home datasets are the union of its inputs' (E0G-5.2), it is recomposable only if every input is (E0G-5.3), and its recorded additivity is non-additive, because a derived value is never re-aggregated (DER-4.2, DER-4.4). |
