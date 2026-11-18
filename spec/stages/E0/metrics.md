# E0.g Classify metrics

## E0G-1 Components

A base metric expression is a scalar expression over one or more aggregates:
`SUM(invoices.amount) / COUNT(DISTINCT customers.customer_id)`.

- **E0G-1.1** Each aggregate is a **component**. Its **home dataset** is the single dataset its arguments read.
- **E0G-1.2** Each component is evaluated over the rows of its own home dataset; the scalar expression is applied afterwards. `AVG(customers.age)` is the average age of customers in each group, whatever joins the query makes.
- **E0G-1.3** The guarantee is that joins never change an aggregate: every row of a home dataset contributes once to its component (Apache Ossie Layer 2, G2).
- **E0G-1.4** In a dataset-scoped metric of the development format (section 4.5), every reference reads the metric's own dataset (a bare field name, or `source.column`; E0D-3), so the home dataset of each of its components is that dataset, also for an aggregate that reads no column (`COUNT(*)`).

## E0G-2 Rules

These are checked at load and recorded on the metric. A metric that breaks one is unusable (E0-4.1); the code is raised by E2 when a query names the metric, in the precedence of E2-5.

| ID | Rule | Code |
|---|---|---|
| E0G-2.1 | A field reference outside every aggregate is an error. | `E_UNAGGREGATED_FINER_GRAIN_REFERENCE` |
| E0G-2.2 | A window function in a metric is an error. | `E_DEFERRED_WINDOWED_METRIC` |
| E0G-2.3 | Aggregate arguments that read more than one dataset are an error. | `E_AMBIGUOUS_MEASURE_GRAIN` |
| E0G-2.4 | A `FILTER (WHERE …)` that reads a dataset other than its arguments' is an error. | `E_AMBIGUOUS_MEASURE_GRAIN` |
| E0G-2.5 | An aggregate whose arguments read no dataset (`COUNT(*)`, `SUM(1)`) is an error in a model-level metric, whatever its `FILTER` reads. | `E_AMBIGUOUS_MEASURE_GRAIN` |
| E0G-2.6 | Nested aggregates are an error. | `E0F-07` |
| E0G-2.7 | A base metric with no aggregate is an error. | `E0G-02` |

Load warning: `W0G-01` for E0G-2.1, E0G-2.3–E0G-2.5 and E0G-2.7; E0G-2.2 and E0G-2.6 follow E0G-2.8.

- **E0G-2.8** E0.g classifies only the metrics that are usable after E0.f. E0G-2.2 and E0G-2.6 are detected earlier, where E0F-2.4 and `E0F-07` report them; E0.g does not report them again. For the other rules, a metric gets one `W0G-01`, located at the metric; when it breaks several, the recorded query code is the first of them in the precedence of E2-5. So `orders.amount` alone gives `E_UNAGGREGATED_FINER_GRAIN_REFERENCE` (E0G-2.1), and E0G-2.7 is reached only by an expression with neither an aggregate nor a field reference, such as `1 + 2`.

## E0G-3 Decomposability

Classes follow the Apache Ossie decomposability reference:

| Class | Functions | Recomposable from partial results |
|---|---|---|
| Distributive | `SUM`, `COUNT`, `MIN`, `MAX` | Yes, with the same function (`COUNT` recomposes by `SUM`) |
| Algebraic | `AVG`, `STDDEV*`, `VAR*` | From distributive parts. Version 1 recomposes only `AVG` (= `SUM` / `COUNT`); the others are computed from base rows |
| Holistic | `MEDIAN`, `PERCENTILE_*`, `COUNT(DISTINCT)`, `SUM(DISTINCT)` | No: always from base rows |

- **E0G-3.1** A metric is recomposable only if all its components are.
- **E0G-3.2** `FILTER (WHERE …)` does not change the class.

## E0G-4 Additivity

Additivity (additive, semi-additive, non-additive) is a separate axis: over which dimensions a
metric may be re-aggregated. **Deduction:** a metric whose components are all distributive is
additive; one with an algebraic or holistic component is non-additive (it is always computed from
base rows at the query grain). Only the [additivity extension](../../extensions/additivity.md) makes a
metric semi-additive.

- **E0G-4.1** A declaration may only restrict what is deduced, never relax it: declaring `semi_additive` on a metric deduced non-additive relaxes it (`E0G-01`); declaring `non_additive` restricts it.
- **E0G-4.2** A semi-additive metric MUST have all its components on one home dataset (`E0G-01`).
- **E0G-4.3** E0G-4.1 and E0G-4.2 are checked in E0.g, against the additivity declaration that E0.c has validated; a metric without a declaration has only its deduced additivity.

## E0G-5 Derived and context metrics

- **E0G-5.1** Dependencies between metrics form a graph that is sorted topologically; a cycle is an error (specified in the derived-metric extension).
- **E0G-5.2** The home datasets of a derived metric are the union of those of its inputs.
- **E0G-5.3** A derived metric is recomposable only if every input is.
- **E0G-5.4** E0G-5.2 and E0G-5.3 apply once the derived-metric extension declares the inputs; they are implemented with it.

## E0G-6 Output

For each metric: its components (aggregate, home dataset, class), additivity, result type and
nullability.

## E0G-7 Codes

| Code | Condition |
|---|---|
| `E0G-01` | An additivity declaration that relaxes the deduced additivity (E0G-4.1), or a semi-additive metric whose components are on more than one home dataset (E0G-4.2) |
| `E0G-02` | A base metric with neither an aggregate nor a field reference (E0G-2.7) |
