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
        data: '{"namespace": "semantic_compiler", "version": 1, "derived": {"on_division_by_zero": "null"}}'
```

## DER-3 Schema

| Key | Required | Meaning |
|---|---|---|
| `on_division_by_zero` | If the formula divides | `null` (that division yields NULL) or `error` (execution fails with `RDER-01`) |

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
