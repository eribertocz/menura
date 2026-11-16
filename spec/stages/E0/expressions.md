# E0.f Type expressions

## E0F-1 Scope

Every expression of the model is parsed and typed against the catalog schema, so that no type
error reaches a query (P6): field expressions, metric expressions, and every expression inside
an extension (population conditions, row policies, summary definitions).

## E0F-2 Language

- **E0F-2.1** Expressions are tagged `ANSI_SQL` in Apache Ossie 0.1.1 and are interpreted with the semantics of the Apache Ossie expression language `OSSIE_SQL_2026`.
- **E0F-2.2** The compiler accepts a **closed subset** of that language, because it cannot guarantee a result it cannot type and translate identically (P1). An expression outside the subset makes its element unusable (E0-4.1).

| Category | Accepted |
|---|---|
| Literals | Text, number, boolean, `DATE '…'`, `TIMESTAMP '…'`, `NULL` |
| References | `dataset.field` in metrics; `source` columns in field expressions; `AGG(metric)` only in derived and context metrics; `USER_ATTRIBUTE('name')` only in row policies |
| Operators | Arithmetic (including `%`), comparison, `AND`, `OR`, `NOT`, `IS [NOT] NULL`, `IS [NOT] DISTINCT FROM`, `[NOT] IN (list)`, `BETWEEN`, `LIKE`, `ILIKE`, `||` |
| Conditionals | `CASE`, `COALESCE`, `NULLIF` |
| Conversion | `CAST(x AS type)` with the `OSSIE_SQL_2026` target types |
| Scalar functions | Those marked REQUIRED in `OSSIE_SQL_2026` for text, numbers, dates and conditionals |
| Aggregates | `SUM`, `SUM(DISTINCT)`, `COUNT`, `COUNT(*)`, `COUNT(DISTINCT)`, `AVG`, `MIN`, `MAX`, `STDDEV*`, `VAR*`, `MEDIAN`, `PERCENTILE_CONT`, `PERCENTILE_DISC`, each with optional `FILTER (WHERE …)`; only in metrics and summary definitions |

- **E0F-2.3** Forbidden everywhere: subqueries, non-deterministic functions and engine-specific functions. `CURRENT_DATE` and `CURRENT_TIMESTAMP` are allowed and are resolved at compilation from the query's `reference_time` and `time_zone`, never from a clock; a query that needs them without `reference_time` fails with `EQRY-03`.
- **E0F-2.4** Window functions are part of `OSSIE_SQL_2026` but not of version 1; in metrics they produce `E_DEFERRED_WINDOWED_METRIC`, as in Apache Ossie Layer 3.
- **E0F-2.5** Numeric semantics follow Apache Ossie: `/` is true division, also between integers; `ROUND` rounds half away from zero.

## E0F-3 Implicit conversions

A conversion is implicit only if it cannot lose information. Everything else requires `CAST`.

| From | To | Implicit |
|---|---|---|
| Narrower integer | Wider integer | Yes |
| Integer | Decimal with enough integer digits | Yes |
| `decimal(p,s)` | Decimal with greater or equal precision and scale that keeps every digit | Yes |
| Integer or decimal | Float | No |
| Date | Timestamp | No |
| Anything | Text, or text to anything | No |

## E0F-4 Relationship keys

- **E0F-4.1** Each pair in `from_columns` / `to_columns` MUST have the same logical kind. Integers may differ in width; decimals MUST have the same scale.

## E0F-5 Result types of aggregates

The compiler fixes the result type, and E6 enforces it with explicit casts, so that the same
metric returns the same type from a base relation and from a summary.

| Aggregate | Argument | Result |
|---|---|---|
| `COUNT`, `COUNT(DISTINCT)` | any | 64-bit integer |
| `SUM` | integer | 128-bit integer |
| `SUM` | `decimal(p,s)` | `decimal(38,s)` |
| `SUM`, `AVG` | float | 64-bit float |
| `AVG` | integer or decimal | 64-bit float, always computed as exact `SUM` divided by `COUNT` |
| `STDDEV*`, `VAR*` | numeric | 64-bit float |
| `MEDIAN`, `PERCENTILE_*` | integer or decimal | 64-bit float; the argument is cast to float first |
| `MIN`, `MAX` | any | argument type |

Rationale, verified on DuckDB: computing `AVG` as exact sum over count keeps the only
approximation in the final division and yields identical values from base and summary;
`MEDIAN` over a decimal keeps the decimal scale and rounds the interpolation (10.00 and 10.01
give 10.00), which casting to float avoids.

## E0F-5a Result types of scalar expressions

The compiler fixes these types, and E6 enforces them with casts, whatever the engine infers.

| Expression | Operands | Result |
|---|---|---|
| `+`, `-`, `*`, `%` | integers | 128-bit integer |
| `+`, `-`, `%` | `decimal(p1,s1)`, `decimal(p2,s2)` (an integer counts as `decimal(38,0)`) | `decimal(min(38, max(p1−s1, p2−s2) + max(s1,s2) + 1), max(s1,s2))` |
| `*` | `decimal(p1,s1)`, `decimal(p2,s2)` | `decimal(min(38, p1+p2), s1+s2)`; `s1+s2 > 38` is `E0F-08` |
| `/` | integers or decimals | 64-bit float (true division, E0F-2.5) |
| any arithmetic | at least one float | 64-bit float |
| `CASE`, `COALESCE`, `NULLIF` | branch types | The branch type to which every other branch converts implicitly (E0F-3); none is `E0F-03` |
| `ROUND(x, n)` | `decimal(p,s)`, literal `0 ≤ n ≤ s` | `decimal(p, n)` |
| `ROUND(x, n)` | float | 64-bit float |
| `ROUND(x, n)` | integer, literal `n ≤ 0` | 128-bit integer |
| `ROUND(x, n)` | non-literal `n`, or decimal with `n > s` | `E0F-03` |
| unary `-` | any numeric | Operand type (integers widen to 128-bit) |
| Scalar functions | — | The result type the `OSSIE_SQL_2026` function reference gives for the argument types; a function whose result type is not fixed there is outside the subset |

## E0F-6 Division

- **E0F-6.1** In every expression except derived metrics, division by zero yields NULL. The generated SQL guarantees it, whatever the engine does (DuckDB returns `inf`). Derived metrics follow their own `on_division_by_zero` rule.

## E0F-7 Nullability

- **E0F-7.1** Every expression carries whether it can be NULL, derived from `nullable` in the catalog. E0.g and E4 use it.

## E0F-8 Diagnostics

| Code | Condition |
|---|---|
| `E0F-01` | Expression cannot be parsed |
| `E0F-02` | Construct, function or operator outside the accepted subset |
| `E0F-03` | Incompatible types without explicit `CAST` |
| `E0F-04` | Use of an `unsupported` column |
| `E0F-05` | Relationship key pair with incompatible types |
| `E0F-06` | Expression that must be boolean is not |
| `E0F-07` | Aggregate where none is allowed, or nested aggregate |
| `E0F-08` | Decimal multiplication whose scale exceeds 38 |
| `W0F-01` | An `E0F` problem; the element is unusable (E0-4.1) |
