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
| Literals | Text, number, boolean, `DATE '…'`, `TIME '…'`, `TIMESTAMP '…'`, `TIMESTAMP_NTZ '…'`, `NULL` (E0F-9.3) |
| References | `dataset.field` in metrics; `source` columns in field expressions; `AGG(metric)` only in derived and context metrics; `USER_ATTRIBUTE('name')` only in row policies |
| Operators | Arithmetic (including `%`), comparison (including `!=`), `AND`, `OR`, `NOT`, `IS [NOT] NULL`, `IS [NOT] DISTINCT FROM`, `[NOT] IN (list)`, `BETWEEN`, `LIKE`, `ILIKE`, `||` |
| Conditionals | Simple and searched `CASE`, and the conditional functions of E0F-10.4 |
| Conversion | `CAST(x AS type)` with the target types of E0F-11 |
| Scalar functions | Exactly those of E0F-10, with the arguments it gives |
| Aggregates | `SUM`, `SUM(DISTINCT)`, `COUNT`, `COUNT(*)`, `COUNT(DISTINCT)`, `AVG`, `MIN`, `MAX`, `STDDEV`, `STDDEV_POP`, `STDDEV_SAMP`, `VARIANCE`, `VAR_POP`, `VAR_SAMP`, `MEDIAN`, `PERCENTILE_CONT(p) WITHIN GROUP (ORDER BY x)`, `PERCENTILE_DISC(p) WITHIN GROUP (ORDER BY x)`, each with optional `FILTER (WHERE …)`; only in metrics and summary definitions (E0F-10.5) |

- **E0F-2.3** Forbidden everywhere: subqueries, non-deterministic functions and engine-specific functions. `CURRENT_DATE` and `CURRENT_TIMESTAMP` are allowed and are resolved at compilation from the query's `reference_time` and `time_zone`, never from a clock; a query that needs them without `reference_time` fails with `EQRY-03`.
- **E0F-2.4** Window functions are part of `OSSIE_SQL_2026` but not of version 1; in metrics they produce `E_DEFERRED_WINDOWED_METRIC`, as in Apache Ossie Layer 3.
- **E0F-2.5** Numeric semantics follow Apache Ossie: `/` is true division, also between integers; `ROUND` rounds half away from zero.
- **E0F-2.6** Everything else the language allows is outside the subset (`E0F-02`): `TRY_CAST`, the RECOMMENDED and EXPERIMENTAL functions, dialect functions, and any function or form E0F-10 does not list. The REQUIRED functions the subset leaves out are deviation DEV-10.

The source of E0F-9 to E0F-11 is Apache Ossie `core-spec/expression_language.md`,
`core-spec/grammar/OssieExpression.g4` and `compliance/expressions/` at the tracked commit
(section 4.1). Where Apache Ossie leaves a result to the engine, these sections fix it, and
E6 generates SQL that gives that result.

## E0F-9 Syntax

- **E0F-9.1** The grammar `OssieExpression.g4` decides what parses: a text it rejects is `E0F-01`; a text it accepts with a construct outside the subset is `E0F-02`. The official expression suite runs in CI: no line of `valid.txt` gives `E0F-01`, and every line of `invalid.txt` does.
- **E0F-9.2** Precedence, tightest first: unary `+` and `-`; `*`, `/`, `%`; `+`, `-`, `||` (one level); comparisons, `[NOT] BETWEEN`, `[NOT] IN`, `[NOT] LIKE`, `[NOT] ILIKE`, `IS [NOT] NULL`, `IS [NOT] DISTINCT FROM` (one level, never chained); `NOT`; `AND`; `OR`. Binary operators of one level associate to the left.
- **E0F-9.3** Lexical rules:

| Element | Rule |
|---|---|
| Keywords and function names | Case-insensitive |
| `!=` | Means `<>` |
| `-- …` and `/* … */` | Whitespace; block comments do not nest |
| Identifiers | Regular or quoted (`"…"`, `""` escapes `"`); compared as E0D-1.1 |
| Text literal | `'…'`, `''` escapes `'` |
| Number with digits only | Exact integer: 64-bit integer up to 18 digits, `decimal(n,0)` for 19 to 38 digits; more digits is `E0F-02` |
| Number with `.` and no exponent | `decimal(p,s)` with exactly its digits (`.5` is `decimal(1,1)`, `10.` is `decimal(2,0)`); `p > 38` is `E0F-02` |
| Number with exponent | 64-bit float |
| `TRUE`, `FALSE`, `NULL` | Boolean and null literals |
| `DATE '…'` | `YYYY-MM-DD`, a real calendar date, years 0001 to 9999 |
| `TIME '…'` | `HH:MM:SS` with an optional fraction of 1 to 6 digits |
| `TIMESTAMP '…'`, `TIMESTAMP_NTZ '…'` | `YYYY-MM-DD HH:MM:SS` with an optional fraction of 1 to 6 digits; both are a timestamp without time zone, precision `us` |

Any other content of a typed literal is `E0F-01`.

- **E0F-9.4** The call forms of the grammar are accepted only where E0F-10 lists them: `*` only in `COUNT(*)`; `DISTINCT` only in `SUM` and `COUNT` with one argument; `WITHIN GROUP` only on `PERCENTILE_CONT` and `PERCENTILE_DISC`, with exactly one sort item; `FILTER (WHERE …)` on every aggregate; `OVER` as E0F-2.4. Anything else is `E0F-02`.

## E0F-10 Functions

Types: *integer*, *decimal*, *float*, *numeric* (any of the three), *text*, *boolean*, *date*,
*time*, *timestamp*. "CASE rule" is the result type of `CASE` in E0F-5a. Unless a row says
otherwise, a NULL argument gives NULL.

### E0F-10.1 Date and time

| Function | Arguments | Result | Semantics |
|---|---|---|---|
| `CURRENT_DATE`, `CURRENT_TIME`, `CURRENT_TIMESTAMP`, with or without `()` | — | date, time, timestamp | E0F-2.3 |
| `YEAR`, `QUARTER`, `MONTH`, `DAY`, `DAYOFYEAR` | date or timestamp | 64-bit integer | |
| `HOUR`, `MINUTE`, `SECOND` | time or timestamp | 64-bit integer | `SECOND` has no fraction |
| `EXTRACT(part FROM x)`, `DATE_PART('part', x)` | `part`: `YEAR`, `QUARTER`, `MONTH`, `WEEK`, `DAY`, `DAYOFWEEK`, `DAYOFYEAR`, `HOUR`, `MINUTE`, `SECOND`; `x` as for the function of the same name | 64-bit integer | `WEEK` is the ISO 8601 week (1 to 53); `DAYOFWEEK` is ISO (Monday 1 to Sunday 7). In `DATE_PART` the part is a text literal, case-insensitive |
| `DATE_TRUNC('part', x)` | `part`: `year`, `quarter`, `month`, `week`, `day` for a date, and also `hour`, `minute`, `second` for a timestamp, as a text literal | type of `x` | A week starts on Monday |
| `DATEADD(part, n, x)` | `part` as for `DATE_TRUNC`, as a bare word or a text literal; `n` integer; `x` date or timestamp | type of `x` | Adding months, quarters or years to a day the target month lacks gives that month's last day |
| `DATEDIFF(part, start, end)` | `part` as for `DATEADD`; `start` and `end` of one type | 64-bit integer | The number of `part` boundaries crossed from `start` to `end`, negative when `end` is earlier; weeks start on Monday |
| `TO_DATE(s)`, `TO_TIMESTAMP(s)` | text | date, timestamp | As `CAST(s AS DATE)` and `CAST(s AS TIMESTAMP)`; the forms with a format argument are outside the subset |

### E0F-10.2 Text

| Function | Arguments | Result | Semantics |
|---|---|---|---|
| `CONCAT(a, …)` | one or more text | text | NULL if any argument is NULL, like `||` |
| `LENGTH(s)` | text | 64-bit integer | Characters (Unicode code points) |
| `LOWER(s)`, `UPPER(s)` | text | text | Unicode simple case mapping: each character maps to one character, so `UPPER('ß')` is `ß` |
| `TRIM(s)`, `LTRIM(s)`, `RTRIM(s)` | text | text | Remove spaces (U+0020) only |
| `LEFT(s, n)`, `RIGHT(s, n)` | text, integer | text | The first or last `n` characters; `n < 0` gives NULL |
| `SUBSTRING(s, start[, length])` | text, integer, integer | text | The characters at positions `start` to `start + length − 1` that exist (ISO 9075); `length < 0` gives NULL |
| `REPLACE(s, from, to)` | text, text, text | text | Every occurrence of `from`, left to right, without overlap; an empty `from` leaves `s` unchanged |
| `SPLIT_PART(s, delimiter, n)` | text, text, integer literal `n ≥ 1` | text | The `n`-th part; empty text when there are fewer parts. An empty `delimiter` leaves `s` as its only part |
| `POSITION(a IN s)`, `CHARINDEX(a, s)` | text, text | 64-bit integer | 1-based position of the first occurrence; 0 when absent; 1 when `a` is empty |
| `CONTAINS(s, a)`, `STARTSWITH(s, a)`, `ENDSWITH(s, a)` | text, text | boolean | |
| `LIKE`, `ILIKE` (operators) | text, text | boolean | `%` and `_` wildcards, no escape character; `ILIKE` compares with Unicode simple case folding |

### E0F-10.3 Numbers

| Function | Arguments | Result | Semantics |
|---|---|---|---|
| `ABS(x)` | numeric | Type of `x` (integers widen to 128-bit) | |
| `SIGN(x)` | numeric | 64-bit integer | −1, 0 or 1 |
| `FLOOR(x)`, `CEIL(x)`, `CEILING(x)` | numeric | integer: 128-bit integer; `decimal(p,s)`: `decimal(min(38, p − s + 1), 0)`; float: 64-bit float | |
| `ROUND(x[, n])` | as E0F-5a | as E0F-5a | One argument means `n = 0` |
| `TRUNC(x[, n])`, `TRUNCATE(x[, n])` | as `ROUND` | as `ROUND` | Toward zero |
| `MOD(x, y)` | as `%` | as `%` | The remainder has the sign of `x`; `y = 0` gives NULL, also for `%` |
| `POWER(x, y)`, `SQRT(x)`, `EXP(x)`, `LN(x)`, `LOG(base, x)`, `LOG10(x)` | numeric | 64-bit float | An argument outside the function's domain gives NULL |
| `GREATEST(a, b, …)`, `LEAST(a, b, …)` | two or more of one comparable type (CASE rule) | CASE rule | NULL if any argument is NULL |

### E0F-10.4 Conditionals

| Function | Arguments | Result | Semantics |
|---|---|---|---|
| `IF(c, a, b)`, `IFF(c, a, b)` | boolean, any, any | CASE rule | `b` when `c` is false or NULL |
| `NULLIF(a, b)` | two of one comparable type | Type of `a` | |
| `COALESCE(a, b, …)`, `IFNULL(a, b)`, `NVL(a, b)` | two or more | CASE rule | The first non-NULL argument |
| `NVL2(a, b, c)` | any, any, any | CASE rule of `b` and `c` | `b` when `a` is not NULL, else `c` |
| `ZEROIFNULL(x)` | numeric | Type of `x` | 0 when `x` is NULL |
| `NULLIFZERO(x)` | numeric | Type of `x` | NULL when `x` is 0 |

### E0F-10.5 Aggregates

The aggregates of E0F-2.2, with the result types of E0F-5. `PERCENTILE_CONT(p)` and
`PERCENTILE_DISC(p)` take a numeric literal `0 ≤ p ≤ 1`; `ORDER BY x DESC` computes the
percentile over the values in descending order; `NULLS FIRST` and `NULLS LAST` are accepted
and have no effect, because these aggregates ignore NULLs.

## E0F-11 `CAST` target types

| Target | Type |
|---|---|
| `VARCHAR`, `STRING` | text |
| `INTEGER`, `INT` | 32-bit signed integer |
| `BIGINT` | 64-bit signed integer |
| `DECIMAL(p, s)`, `NUMERIC(p, s)`; `DECIMAL(p)` means scale 0 | decimal, `1 ≤ p ≤ 38`, `0 ≤ s ≤ p` |
| `FLOAT`, `DOUBLE` | 64-bit float |
| `BOOLEAN`, `DATE`, `TIME` | boolean, date, time |
| `TIMESTAMP`, `TIMESTAMP_NTZ` | timestamp without time zone, precision `us` |

- **E0F-11.1** Any other type name, `DECIMAL` without precision, and parameters on any other type are `E0F-02`.
- **E0F-11.2** Text converts only from the ISO forms of E0F-9.3. For a number, that is a number literal form of E0F-9.3, optionally preceded by one `-` and with no spaces. A conversion that fails, or a value that does not fit, is an execution error, never NULL. A conversion to fewer fractional digits rounds half away from zero, like `ROUND`.

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

- **E0F-3.1** Comparisons, `[NOT] IN`, `[NOT] BETWEEN`, `IS [NOT] DISTINCT FROM` and the `WHEN` values of a simple `CASE` (each compared with the `CASE` operand) accept any two numeric operands: they are compared in the type E0F-5a gives their `-` (exact for integers and decimals, float when either is a float). Other operands follow the CASE rule (E0F-5a).
- **E0F-3.2** The literal `NULL` takes the type of another operand of the same operator or function wherever the result type follows the operands. Where no other operand gives a type (`ABS(NULL)`, `-NULL`, `SUM(NULL)`, `MIN(NULL)`, `DATE_TRUNC('day', NULL)`), it is `E0F-03`; a `CAST` gives it one. A function with a fixed result type accepts `NULL`. An expression that is only the literal `NULL` has no other operand, so it is `E0F-03`; `CAST(NULL AS t)` gives it type `t`.

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
| `MEDIAN`, `PERCENTILE_*` | integer, decimal or float | 64-bit float; an integer or decimal argument is cast to float first |
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
| `*` | `decimal(p1,s1)`, `decimal(p2,s2)` (an integer counts as `decimal(38,0)`) | `decimal(min(38, p1+p2), s1+s2)`; `s1+s2 > 38` is `E0F-08` |
| `/` | integers or decimals | 64-bit float (true division, E0F-2.5) |
| any arithmetic | at least one float | 64-bit float |
| `CASE`, `COALESCE`, `NULLIF` | branch types | The branch type to which every other branch converts implicitly (E0F-3); none is `E0F-03` |
| `ROUND(x, n)` | `decimal(p,s)`, literal `0 ≤ n ≤ s` | `decimal(p, n)` |
| `ROUND(x, n)` | float | 64-bit float |
| `ROUND(x, n)` | integer, literal `n ≤ 0` | 128-bit integer |
| `ROUND(x, n)` | non-literal `n`; decimal with `n > s` or `n < 0`; integer with `n > 0` | `E0F-03` |
| `SPLIT_PART`, `PERCENTILE_*` | a non-literal or out-of-range `n` or fraction (E0F-10.2, E0F-10.5) | `E0F-03` |
| unary `-` | any numeric | Operand type (integers widen to 128-bit) |
| Scalar functions | As E0F-10 | As E0F-10 |

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
