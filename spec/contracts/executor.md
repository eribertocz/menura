# Executor: running a compiled plan

Rules for every executor that runs a compiled plan ([contract](compiled-plan.md)) and returns
its result in Apache Arrow (architecture section 3.1). This chapter adds no document and no
JSON Schema: it states what an executor does with the plan it receives, so that the rows it
returns are exactly the ones the plan means. The executor never modifies `sql` or `parameters`
(PLN-2.5). Its rules join the coverage scope at the gate of the release that implements them
(7.1a).

## EXE-1 Engine and connection

| ID | Rule |
|---|---|
| EXE-1.1 | **Engine versions.** A plan is valid for the exact DuckDB version (E6-5.4) and the exact ClickHouse version (E6-7.10) pinned by the Menura release that produced it (`producer.compiler_version`), which the compatibility matrix of that release records (CI-4.3). An executor that runs a plan on another engine version does so at its own risk: the specification claims nothing about the result (CI-4.4). |
| EXE-1.2 | **DuckDB extensions.** The SQL of a `duckdb` plan needs no DuckDB extension other than `icu`, which provides time zones: this bounds what the plan's SQL needs, not what the executor loads to read its sources (EXE-1.5). The executor runs the plan on a DuckDB build that includes `icu`, so no setting, statement or query ever downloads or installs an extension at run time. The SQL never loads or installs one. |
| EXE-1.3 | **Data source.** The executor maps `data_source` to a connection of its own, with its own credentials (PLN-1). For the `clickhouse` dialect, the connection's user may change the settings of the statement's `SETTINGS` clause per query (E6-7.8). |
| EXE-1.4 | **Relations in DuckDB.** In a `duckdb` plan, the first part of a relation `source` of three parts (`catalog.schema.table`, E6-4.3) is the exact name of a database that the executor attaches to the connection under that name, spelled as the catalog spells it, before it executes the plan. A `source` of two parts (`schema.table`) names a schema and a table of the connection's default database. The SQL never attaches, detaches or switches a database (PLN-2.3). |
| EXE-1.5 | **Storage extensions.** When it prepares its connection, before it executes the plan, the executor MAY load the DuckDB extensions it needs to read its sources (for example `ducklake`, `parquet`, or an extension for object-storage access). It loads them only from its own build, never downloaded at run time (EXE-1.2). Such an extension never sets a session setting the result depends on (E6-4.10) and never replaces a function the plan's SQL calls. |

## EXE-2 Parameters

| ID | Rule |
|---|---|
| EXE-2.1 | **Binding.** The executor binds the parameter at position n to `$n` in a `duckdb` plan (E6-4.2) and to the query parameter `pn` in a `clickhouse` plan (`param_pn` over HTTP, E6-7.3). It binds the value of the table of EXE-2.3 for the parameter's `logical_type`, read from its CON-4 form. The bound value is exactly the value the CON-4 form denotes: it is never rounded or truncated, never converted through a binary float unless the parameter is itself a float, and never read in a session time zone or under any other session setting (E6-4.10). Both dialects type every use of a parameter (E6-4.2c, E6-7.3), so the executor never chooses a type: it only binds a value that the type reads exactly. |
| EXE-2.2 | **NULL.** A parameter whose `value` is `null` is bound as SQL NULL in DuckDB, whatever its type, and as `\N` in ClickHouse (E6-7.3). |
| EXE-2.3 | **Values by type.** The native value of each logical type is the one of the table below. A type with no ClickHouse value is never a parameter of a `clickhouse` plan, because compilation fails with `E6-01` (E6-7.12). `binary` and `interval` have no CON-4 encoding, so no parameter has either type. |
| EXE-2.4 | **No NaN.** No parameter is NaN or infinite: CON-4 writes a float as a JSON number, and JSON has neither. |
| EXE-2.5 | **Fraction digits.** A `time` or `timestamp` parameter holds at most six fraction digits: a query literal has at most six (E0F-9.3), `reference_time` keeps microseconds (QRY-2), and no security attribute has either type (SCY-2.2). A binding at microsecond resolution is therefore exact for every precision, `ns` included. |

| Logical type | CON-4 value | DuckDB (`$n`) | ClickHouse (`pn`) |
|---|---|---|---|
| `boolean` | `true`, `false` | `BOOLEAN` | `true` or `false` |
| `integer`, `bits` b, `signed` | JSON number, or a string outside ±(2^53 − 1) | The integer of that width and signedness (`TINYINT` to `HUGEINT`, `UTINYINT` to `UHUGEINT`, CAT-4.4), made from the exact integer, never through a float | Its decimal digits, with a leading `-` when negative, as CON-4 writes them |
| `decimal(p,s)` | String | `DECIMAL(p,s)` made from the unscaled integer (the digits without the `.`), never a `DOUBLE` | The CON-4 string, unchanged, which `Decimal(p, s)` reads exactly |
| `float`, `bits` 32 | JSON number | `FLOAT` of the 32-bit value the number denotes | A decimal text that reads back as exactly that 32-bit value |
| `float`, `bits` 64 | JSON number | `DOUBLE` of the value the number denotes | A decimal text that reads back as exactly that 64-bit value |
| `text` | String | `VARCHAR` | The text in ClickHouse's escaped form: `\`, tab, line feed and carriage return written `\\`, `\t`, `\n` and `\r` |
| `date` | `YYYY-MM-DD` | `DATE` of that day of the proleptic Gregorian calendar | The CON-4 text, which `Date32` reads |
| `time` | `HH:MM:SS[.f]` | `TIME` of the microseconds since midnight. A query literal has at most six fraction digits (E0F-9.3) | None (E6-7.12) |
| `timestamp`, `precision` p, without time zone | `YYYY-MM-DDTHH:MM:SS[.f]` | A timestamp of the wall-clock value: `TIMESTAMP` (microseconds), exact for every precision (EXE-2.5) | None (E6-7.12) |
| `timestamp`, `precision` p, with time zone | `YYYY-MM-DDTHH:MM:SS[.f]Z` | `TIMESTAMP WITH TIME ZONE` of the microseconds since `1970-01-01T00:00:00Z`; or, where the binding interface has no such value, a `VARCHAR` holding the CON-4 text with `Z` replaced by `+00:00`. Never a `TIMESTAMP`, which the cast would read in the session `TimeZone` | The CON-4 text with `T` replaced by a space and `Z` removed, which `DateTime64(<d>, 'UTC')` reads as an instant in UTC |
| `uuid` | Lowercase, hyphenated | `UUID`, or a `VARCHAR` holding the CON-4 text | The CON-4 text |
| `list<T>` | JSON array | A list whose elements are bound as `T`'s row states, a NULL element as NULL (E6-5.3) | `[e1,e2,…]`: a NULL element is `NULL`; a `boolean`, `integer` or `float` element is its text; any other element is its text between single quotes, escaped as in the `text` row and with each `'` written `\'` |
| `binary`, `interval` | None (CON-4) | None | None |

## EXE-3 Result

| ID | Rule |
|---|---|
| EXE-3.1 | **Schema.** The result is Apache Arrow with one field per `result_schema` column, in `result_schema` order: the field's name is the column's `name`, its type is given by the column's `logical_type` in the table below, and it is nullable exactly when the column's `nullable` is true. The transport (an IPC stream, the C data interface, Arrow Flight) and the batch sizes are the executor's choice. |
| EXE-3.2 | **Values and order.** Every value is the exact value the engine returns, converted to its Arrow type without rounding, truncation or a change of time zone. Rows keep the order in which the statement returns them, which its `ORDER BY` fixes (E6-4.8). |
| EXE-3.3 | **Precision.** A `timestamp` precision maps to the Arrow time unit of the same name: `s` to seconds, `ms` to milliseconds, `us` to microseconds and `ns` to nanoseconds. |
| EXE-3.4 | **NaN.** A float NaN in a result is an IEEE NaN in Arrow, and an infinite float is the IEEE infinity of its sign; neither is NULL or an error. Comparisons, grouping and ordering in the statement treat NaN as E6-7.14 states, in both dialects: NaN equals NaN and sorts after every other value. The executor never regroups or reorders rows (EXE-3.2), so a NaN keeps the place the statement gave it. |

| Logical type (CAT-4.1) | Arrow type |
|---|---|
| `boolean` | `Boolean` |
| `integer`, `bits` 8, 16, 32 or 64, `signed: true` | `Int8`, `Int16`, `Int32` or `Int64` |
| `integer`, `bits` 8, 16, 32 or 64, `signed: false` | `UInt8`, `UInt16`, `UInt32` or `UInt64` |
| `integer`, `bits` 128 | `Decimal256(39, 0)`, which holds every value of both signednesses |
| `decimal(p,s)` | `Decimal128(p, s)`. CAT-4.1 caps `precision` at 38, so a decimal never needs `Decimal256` |
| `float`, `bits` 32 or 64 | `Float32` or `Float64` |
| `text` | `Utf8` |
| `binary` | `Binary` |
| `uuid` | `FixedSizeBinary(16)` with the canonical extension type `arrow.uuid`, its 16 bytes in the order of its canonical text |
| `date` | `Date32` |
| `time` | `Time64` with unit microseconds |
| `timestamp`, `precision` p, with time zone | `Timestamp` with the unit of p (EXE-3.3) and time zone `"UTC"` |
| `timestamp`, `precision` p, without time zone | `Timestamp` with the unit of p and no time zone |
| `interval` | `Interval(MonthDayNano)`: its months, its days, and the rest in nanoseconds |
| `list<T>` | `List` whose nullable child field `item` has the Arrow type of `T` |

## EXE-4 Limits

| ID | Rule |
|---|---|
| EXE-4.1 | **Enforced limits.** The executor enforces the plan's `limits` (PLN-1). `timeout_ms` bounds the time from the moment it submits the statement to the engine until the engine has delivered the last row. `max_rows` bounds the number of rows of the result; when it is `null`, the result has no limit beyond the query's own. An executor MAY enforce a lower limit of its own, with the same codes. |
| EXE-4.2 | **No truncated result.** When the result has more rows than `max_rows`, the executor cancels the statement and fails with `REXE-01`. When `timeout_ms` passes before the last row, it cancels the statement and fails with `REXE-02`. Either way it never returns the rows it has as a complete result, and it never stores them under the plan's `cache_key.key`. |

## EXE-5 Runtime errors

| ID | Rule |
|---|---|
| EXE-5.1 | **Raised message.** A runtime failure of the plan (EXT-1) is raised by its SQL with a constant message that starts with the runtime code, a colon and a space: `RDER-01: …` (E6-4.6a) and `RADD-01: …` to `RADD-04: …` (E6-4.6b), through `error(…)` in DuckDB and `throwIf` in ClickHouse (E6-7.5). |
| EXE-5.2 | **Engine error.** The engine reports the raised message inside its own error. DuckDB reports an invalid-input error whose message is `Invalid Input Error: ` followed by the raised message. ClickHouse reports error 395 (`FUNCTION_THROW_IF_VALUE_IS_NON_ZERO`), whose message has ClickHouse's own text before and after the raised message (E6-7.5). |
| EXE-5.3 | **Mapping.** The executor MUST map the raised message to its runtime code by that prefix alone: the characters before the first `: ` of the raised message, of the form `R`, upper-case letters, `-` and two digits. It MUST NOT parse or depend on the rest of the message (the condition and the metric name), and MUST NOT take a code from anywhere else in the engine's error text. An engine error of any other kind, or whose raised message has no such prefix (an overflow, E6-7.9; a failed conversion, E0F-11.2), carries no runtime code of Menura. |
| EXE-5.4 | **Reporting.** How the executor reports a runtime code, an `REXE` code or any other failure to its own clients is the executor's own format, not part of any Menura contract. |

## EXE-6 Time-zone database

| ID | Rule |
|---|---|
| EXE-6.1 | **Engine data.** The engine's time-zone data is the data of the pinned engine version (EXE-1.1). For ClickHouse, which embeds its own data (E6-7.16), the executor reads `TZDATA_VERSION` from `system.build_options` before it executes a plan, and when that version differs from the one the compatibility matrix records for the ClickHouse version in use (CI-4.3), or cannot be read, it fails with `REXE-03` and does not execute the plan. For DuckDB, the time-zone data is fixed by the DuckDB build through its `icu` extension and no statement reports its version, so EXE-1.1 alone governs it and no check applies. This check comes before every other check of this chapter. |
| EXE-6.2 | **Compiler data.** `producer.tzdata_version` names the time-zone data the compiler used (E6-5.5). It may differ from the engine's: the compiler's data decides only which zone names a query may use (E1-2.3) and the bounds of relative ranges, which reach the SQL as bound parameters (QRY-4.2); every other time computation is the engine's. A difference between the two is never `REXE-03`. |

## EXE-7 Data versions

| ID | Rule |
|---|---|
| EXE-7.1 | **Exact versions.** When `cache_key.data_versions` is not `null` (PLN-3.3), the executor MUST NOT return a result of the plan, and MUST NOT store or serve one under its `cache_key.key`, unless it guarantees that the statement read every listed `source` and `period` at exactly the listed `version`. When it cannot guarantee it, it fails with `REXE-04` and returns no result. |
| EXE-7.2 | **Means.** How the executor guarantees it is its own choice: for example, by reading a snapshot or a time-travel version that holds exactly those versions, or by checking the versions before and after execution and failing when they differ from the plan's or changed in between. |
| EXE-7.3 | **No versions.** When `data_versions` is `null`, the result is tied to no version and `REXE-04` never applies; PLN-3.3 governs whether the executor may reuse it. |
| EXE-7.4 | **Cache partitions.** `cache_key.key` deliberately contains neither the compiler version nor the engine version (PLN-3). The compiler version is not needed: a compiler that writes another `sql` or other `parameters` changes `plan_hash` (PLN-3.1), and one that writes the same ones means the same result. `dialect` is already part of `plan_hash`. The engine version is the one the executor runs the plan on, which the plan does not record, and a plan is valid only for the pinned one (EXE-1.1), so the executor MUST keep results produced by different engine versions apart: it MUST NOT serve a result stored under a `cache_key.key` by an engine of another version than the one it would run the plan on. How it keeps them apart (one cache per engine version, or the engine version added to its own lookup key) is its own choice. |

## EXE-8 Diagnostics

| Code | Condition |
|---|---|
| `REXE-01` | The result has more rows than `limits.max_rows` (EXE-4.2) |
| `REXE-02` | Execution did not deliver the last row within `limits.timeout_ms` (EXE-4.2) |
| `REXE-03` | A ClickHouse engine's time-zone data version differs from the one the compatibility matrix records for its version, or cannot be read (EXE-6.1) |
| `REXE-04` | The executor cannot guarantee that it read exactly the plan's `data_versions` (EXE-7.1) |
