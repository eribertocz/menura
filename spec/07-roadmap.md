# 7. Roadmap

Versions follow Semantic Versioning. Before 1.0.0, any minor release may change contracts. From
1.0.0 on, contracts follow the evolution rules of section 3.3.

Each release closes one gate: its stage is merged and every conformance test passes.

## 7.1 Path to the MVP

| Version | Gate | Usable result |
|---|---|---|
| 0.1.0 | Contracts and repository: JSON Schemas, examples, CI with boundary rules A1–A6 and standards | Contracts can be validated |
| 0.2.0 | E0: load and validate the model, catalog and extensions | `semantic-compiler validate` |
| 0.3.0 | E1–E2: analyze and bind JSON and SQL queries | Queries are checked against the model |
| 0.4.0 | E3: join paths and reachability | Ambiguous or missing paths are reported |
| 0.5.0 | E4: logical plan, security (access and row rules), additivity; context modifiers (E4-8, CTX) excluded until 0.8.0 | Correct, secure neutral plan |
| 0.6.0 | E5: optimization and summary selection | Summaries used when valid |
| 0.7.0 | E6: DuckDB generation, lineage, audit, cache key | End-to-end compilation |
| 0.8.0 | Context modifiers: period-to-date, prior period, percent of total | Time intelligence |
| 0.9.0 | Python package, CLI complete, performance targets (R1–R3), full model conformance (OSS-2.1) and Layer 3 conformance; Layer 2 subset per QRY-5 with its remaining cases in the expected-failure list | Release candidate |
| **1.0.0** | **MVP.** Contracts frozen at major version 1 | Production use |

## 7.2 After the MVP

| Version | Content |
|---|---|
| 1.1.0 | ClickHouse dialect |
| 1.2.0 | Window metrics (moving windows, running totals), ranking and top N |
| 1.3.0 | Fiscal and custom calendars |
| 1.4.0 | Metric templates and query parameters |
| 1.x | Native adoption of each new official Apache Ossie capability; matching extensions deprecated |

Modules outside the compiler (executor, natural-language translator, gateway, pipelines) have
their own roadmaps in their own repositories.

## 7.3 Capability scope

| Capability | How it is provided | When |
|---|---|---|
| Query filters, row-level field expressions, basic aggregates | Apache Ossie | MVP |
| Metrics built from metrics | Derived extension | MVP |
| Balances and other semi-additive metrics | Additivity and population extensions | MVP |
| Time grain and date range | Query extension | MVP |
| Choosing between relationships (role-playing dates) | Query `paths` | MVP |
| Dataset, field and row security | Security extension | MVP |
| Precomputed summaries | Summaries extension | MVP |
| Period to date, prior period, share of total | Context extension | MVP |
| Standard calendar | Time fields of the model | MVP |
| Fiscal and custom calendars | Calendar extension: explicit field mapping | After |
| Moving windows, running totals | Window extension (deferred by Apache Ossie) | After |
| Ranking and top N per group | Query extension | After |
| Metric templates applied to many metrics | Template extension | After |
| Query parameters | Query extension | After |
| Display formats | Metadata passed through to hosts | After |
| Relationships declared by the query | Query extension | After |
| Calculations over a displayed result | Host responsibility | Never in the compiler |

## 7.4 Backlog after the MVP

| ID | Item |
|---|---|
| F1 | Moving windows, nested aggregates and ranking |
| F2 | Predicates on metric values (`HAVING`) |
| F3 | Subtotals (`ROLLUP`, `CUBE`, `GROUPING SETS`) |
| F4 | Row listings without aggregation |
| F5 | Per-metric filters and filter exemptions |
| F6 | Window functions in model expressions |
| F7 | Aggregates whose arguments read several datasets |
| F8 | Nested aggregates |
| F9 | One-to-many paths and bridge datasets |
| F10 | Traversing one-to-one relationships in both directions |
| F11 | Several fields in a semi-additive `over` |
| F12 | Path choice declared inside a metric |
| F13 | Custom calendars (for example 4-4-5) and custom grains |
| F14 | Date formatting functions |
| F15 | Summaries for semi-additive metrics, once their full semantics is proven preserved |
| F16 | Combining a summary with recent base rows |
| F17 | Approximate, re-aggregable distinct counts |
| F18 | Recomposing `STDDEV` and `VAR` from components |
| F19 | Value masking |
| F20 | Normative reader for the next official Apache Ossie release, with migration |
| F21 | Dataset-scoped metrics and `AGG(dataset.metric)` in derived metrics |
| F22 | Project-specific `vendor_name` replacing `COMMON` + `namespace` |
| F23 | Apache Ossie Layer 4 (ontology) |
| F24 | Final project name, replacing `semantic_compiler` |
| F25 | ClickHouse dialect (planned for 1.1.0) |
| F26 | Natural-language translator module (separate repository) |
| F27 | Filling time gaps with an explicit time spine |
| F28 | Layer 2 SQL growth, one construct per release with its suite cases: many-to-one `JOIN`, `HAVING`, CTEs and subqueries, other join types, set operators, until full Layer 2 conformance |
