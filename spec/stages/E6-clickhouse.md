# E6 Generate SQL: ClickHouse

From 1.1.0 the dialect `clickhouse` generates SQL for ClickHouse. It uses the plans E1 to E5 produce, unchanged (P5), and the rules of E6-1 to E6-4 and E6-6, with the changes this chapter states. Only the text of `sql`, the names in `result_schema.physical_type` and the parameter form differ from DuckDB, and every query returns the same rows on both engines (E6-7.10). This chapter enters the coverage scope at the 1.1.0 gate (CS-2.6).

## E6-7 Rules

| ID | Rule |
|---|---|
| E6-7.1 | Identifiers are quoted with double quotes, as E6-4.3 requires. A dataset `source` `db.table` names the ClickHouse database and table. |
| E6-7.2 | **Types.** A logical type (CAT-4.1) is written as: `boolean` → `Bool`; `integer` → `Int<bits>` when signed and `UInt<bits>` when not; `decimal(p,s)` → `Decimal(p, s)`; `float` → `Float32` or `Float64`; `text` → `String`; `binary` → `String`; `uuid` → `UUID`; `date` → `Date32`; `timestamp` with time zone → `DateTime64(<d>, 'UTC')`, `<d>` 0, 3, 6 or 9 for precision `s`, `ms`, `us` or `ns`; `list<T>` → `Array(<T>)`; a nullable value wraps its type in `Nullable(…)` where ClickHouse allows it. `result_schema.physical_type` holds these names instead of E6-5.6's. |
| E6-7.3 | **Parameters.** In place of the `$n` form of E6-4.2 and the cast of E6-4.2c, the parameter at position n is written `{pn:<type>}` (`{p1:Int64}`), `<type>` the name of E6-7.2 for its logical type, so the type travels with every use. `parameters` keeps its order and encoding (CON-4); an executor binds position n to the name `pn`. |
| E6-7.4 | In place of `IS NOT DISTINCT FROM` (E6-4.7), keys are compared with a null-safe equality: two NULL values are equal and a NULL value never equals a non-NULL one, in `JOIN … ON` and elsewhere. The SQL form is internal to the generator (E6-4.14) and is proved by execution (E6-7.10). |
| E6-7.5 | In place of `error(…)` (E6-4.6b), a runtime condition is raised with `throwIf`, whose message is the message E6-4.6b states, so the executor's error starts with the code (CS-2.4c). |
| E6-7.6 | **Time.** Grains are truncated in the query time zone with the ClickHouse truncation functions; ISO weeks start on Monday. A `timestamp` value is converted to the query time zone before truncation, and the result is cast back to `Date32` for a `date` field, as E6-5.2 does for DuckDB. |
| E6-7.7 | List parameters test membership with standard NULL semantics: `x IN list` is NULL when `x` is NULL, or when `x` is not in the list and the list holds NULL, and `NOT IN` is its negation (as E6-5.3). |
| E6-7.8 | **Settings in the statement.** Every statement ends with a `SETTINGS` clause that fixes each server setting its meaning depends on, at least `join_use_nulls = 1` (outer joins produce NULL, not default values) and `transform_null_in = 0`; the generator crate documents the full list. A query whose meaning would depend on a setting missing from that clause is a defect (E6-4.10). |
| E6-7.9 | Division, casts and decimal arithmetic follow E6-4.5, E6-4.6 and E6-4.10 with explicit ClickHouse casts, so a result type never depends on ClickHouse's own promotion rules. |
| E6-7.10 | **Verification.** CI executes the generated SQL of every `plan` case on the ClickHouse version pinned by the project, a long-term-support release, with the settings of E6-7.8 set to the opposite value at the session level and a server time zone other than UTC, and compares the rows with the case's expected rows, which are the same for both dialects (CS-2.4). |
| E6-7.11 | Integrity-check queries (E6-6.2) are generated for ClickHouse by the same rules. |
| E6-7.12 | **Types the dialect cannot express.** `time`, `interval`, and `timestamp` without time zone have no ClickHouse type. When the plan reads or returns a value of one of them, compilation fails with `E6-01`, located at the first such field or metric in query order. It is checked after E5 and before any SQL is written, so it never depends on the generated text. |
| E6-7.13 | **Catalog mapping.** A catalog producer for ClickHouse maps physical types as the table below states, reading through `Nullable(T)` and `LowCardinality(T)` to `T` (nullability goes to the column's `nullable`, CAT-4). Every type the table does not list is `unsupported` (CAT-4.3). A `DateTime` or `DateTime64` without an explicit time-zone argument is `unsupported`, because its meaning depends on the server's time zone (P1). |

| ClickHouse type | Logical type (CAT-4.1) |
|---|---|
| `Bool` | `boolean` |
| `Int8`, `Int16`, `Int32`, `Int64`, `Int128` | `integer`, `bits` 8, 16, 32, 64, 128, `signed: true` |
| `UInt8`, `UInt16`, `UInt32`, `UInt64`, `UInt128` | `integer`, `bits` 8, 16, 32, 64, 128, `signed: false` |
| `Decimal(P, S)` with P at most 38 | `decimal`, `precision` P, `scale` S |
| `Float32`, `Float64` | `float`, `bits` 32, 64 |
| `String` | `text` |
| `FixedString(N)` | `binary` |
| `UUID` | `uuid` |
| `Date`, `Date32` | `date` |
| `DateTime('<zone>')`, `DateTime64(P, '<zone>')` with P 0, 3, 6 or 9 | `timestamp`, `precision` `s` (`DateTime`, or P 0), `ms`, `us` or `ns`, `with_time_zone: true` |
| `Array(T)` | `list`, `element` the mapping of `T` |

## E6-8 Errors

| Code | Condition |
|---|---|
| `E6-01` | The plan reads or returns a value whose logical type the target dialect cannot express (E6-7.12). |
