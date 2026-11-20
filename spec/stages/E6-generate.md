# E6 Generate SQL

## E6-1 Purpose

Turn the optimized logical plan into the compiled plan ([contract](../contracts/compiled-plan.md))
for the target dialect. Priorities, in order: correct, then fast, then readable.

## E6-2 Input and output

| | Content |
|---|---|
| Input | Optimized logical plan, dialect (`duckdb`), producer metadata |
| Output | Compiled plan: SQL, parameters, result schema, cache key, limits, lineage, explanation |

## E6-3 Structure of the generator

- **E6-3.1** One generator per dialect, each in its own crate. A generator only translates plan nodes; it never decides semantics.
- **E6-3.2** All dialect knowledge (quoting, function names, casts, truncation, NULL ordering) lives in the generator. No other stage mentions a dialect (S4).

## E6-4 SQL rules (all dialects)

| ID | Rule |
|---|---|
| E6-4.1 | One read-only statement: `SELECT`, optionally with `WITH`. |
| E6-4.2 | Every value from the query or the security context is a positional bound parameter (`$1`, `$2`, …), in order of first appearance. |
| E6-4.2a | A parameter bound from a literal of the query has the literal's own logical type, as E0.f types literals (E0F-2 literal table): `100` is an integer of 64 bits, signed; `1.50` is `decimal(3,2)`; `DATE '2026-01-01'` is `date`; a string is `text`. Its `value` is encoded as CON-4 requires. |
| E6-4.2b | A comparison between a `timestamp` operand and a `date` parameter (the bounds of QRY-4.2) converts the date to the start of that day: for `timestamp with time zone`, local midnight in the query `time_zone` (`CAST($n AS TIMESTAMP) AT TIME ZONE $tz`); without time zone, the same wall-clock midnight (`CAST($n AS TIMESTAMP)`). No implicit cast is left to the engine (E6-4.10). |
| E6-4.3 | Identifiers are always quoted. A dataset `source` that is a relation name is parsed as an SQL qualified name (quoted parts may contain dots) and each part is quoted. A `source` is a query if its text, after leading whitespace, starts with the word `SELECT` or `WITH` (case-insensitive) followed by a character that cannot continue an unquoted identifier (anything but a letter, a digit, `_` or `$`) or by the end of the text; otherwise it is a relation name, so `withdrawals.main.entries` is a relation. A query `source` is emitted as a parenthesized subquery; it MUST be a single read-only `SELECT`, checked at load (`E0D-08`). |
| E6-4.4 | Every plan node becomes a named common table expression in plan order, so the SQL reads top-down and each step is inspectable. |
| E6-4.5 | Every metric result is cast to its fixed result type (E0F-5). |
| E6-4.6 | Every division outside derived metrics is generated as `CASE WHEN d = 0 THEN NULL ELSE n / d END` (E0F-6). |
| E6-4.7 | Stitching compares keys with `IS NOT DISTINCT FROM`, and missing branch rows are detected with a presence column, never by `COALESCE` of a value that can be NULL (Apache Ossie Layer 3, 5.5). |
| E6-4.8 | `ORDER BY` always writes `NULLS FIRST` or `NULLS LAST` explicitly. |
| E6-4.9 | Formatting is fixed: keywords upper case, one clause per line, two-space indentation, no trailing spaces. Identical plans give byte-identical SQL (P3). |
| E6-4.10 | The SQL MUST NOT depend on session settings. Division is generated with explicit operand casts to the result type of E0F-5a; time-zone conversions use `AT TIME ZONE` with the query time zone as a bound parameter; no implicit cast between `timestamp with time zone` and `timestamp` or `date` is ever left to the engine. |
| E6-4.11 | Literals are generated only for values from closed sets or already validated: grain names, `LIMIT` integers, constants written in the model, the boolean constants `TRUE` and `FALSE` that E4 introduces for row policies (E4-6.2, E4-6.4), and `NULL`, which carries no value. Every other value is a bound parameter. |

## E6-5 DuckDB

| ID | Rule |
|---|---|
| E6-5.1 | Identifiers are quoted with double quotes. |
| E6-5.2 | Time grains use `date_trunc('<grain>', x)` evaluated in the query `time_zone`; ISO weeks use `date_trunc('week', x)`, which starts on Monday. For `timestamp with time zone`, the value is first converted to the query time zone. Because DuckDB returns a timestamp when truncating a date, the result is cast back to `DATE` for date fields. |
| E6-5.3 | List parameters are bound as DuckDB lists and tested with `x IN (SELECT unnest($n))` and `x NOT IN (SELECT unnest($n))`, which keep standard NULL semantics (`2 NOT IN (1, NULL)` is NULL). |
| E6-5.4 | The generated SQL is verified in CI by execution on the DuckDB version pinned by the project, with non-default session settings (`TimeZone` not UTC, `integer_division = true`) to prove E6-4.10. |
| E6-5.5 | The compiled plan's `producer` declares the IANA time-zone database version the compiler used to resolve relative ranges; executors SHOULD use the same version. |
| E6-5.6 | `result_schema.physical_type` is the DuckDB name of the column's logical type in the CAT-4.4 table: the first name of its row (`BIGINT`, `VARCHAR`, `TIMESTAMP`, …), `DECIMAL(p,s)` with its numbers, `TIMESTAMP WITH TIME ZONE`, and `<element>[]` for a list (`VARCHAR[]`). |
| E6-5.7 | A parameter bound from a security attribute has the `logical_type` of its declaration (SCY-2.2): `text` → `text`; `integer` → `integer` of 64 bits, signed; `decimal(p,s)` → `decimal` with `precision` p and `scale` s; `date` → `date`; `boolean` → `boolean`; `list<T>` → `list` whose `element` is the mapping of `T`. Its `value` is encoded as CON-4 requires. |

## E6-6 Other outputs

- **E6-6.1** The result schema, cache key, lineage, explanation and audit record are built as specified in PLN-1 to PLN-6.
- **E6-6.2** Integrity-check queries (E0H-4) are generated by this same generator, called by `core`.
