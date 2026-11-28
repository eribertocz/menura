# Extensions

Capabilities that Apache Ossie does not define yet. Each is temporary (section 4.3): when an
official Apache Ossie release covers it, the native form is adopted and the extension is
deprecated and migrated.

| Extension | Key | Attached to | Chapter |
|---|---|---|---|
| Additivity | `additivity` | Metric | [additivity.md](additivity.md) |
| Population | `populations` | Dataset | [population.md](population.md) |
| Derived metrics | `derived` | Metric | [derived.md](derived.md) |
| Context modifiers | `context` | Metric | [context.md](context.md) |
| Security | `security_context`; `access`, `row_access` | Model; Dataset | [security.md](security.md) |
| Summaries | `aggregate` | Dataset | [summaries.md](summaries.md) |
| Validity | `valid_to_bound`, `open_end` of `populations`; `temporal` | Dataset; Relationship | [validity.md](validity.md) |

All entries use the shape of MDL-2 and the common rules of E0.c.

## EXT-1 Runtime failure policies

Some policies (`error` options) can only be decided on the data. The compiler cannot see data
(P2), so the generated SQL fails at execution when the case occurs, instead of returning a
number. In DuckDB this is `error('<code>: <message>')` inside a `CASE`; the code is the
extension's runtime code, so the executor can report it.
