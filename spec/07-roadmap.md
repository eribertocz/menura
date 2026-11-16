# 7. Roadmap

Versions follow Semantic Versioning. Before 1.0.0, any minor release may change contracts. From
1.0.0 on, contracts follow the evolution rules of section 3.3.

Each release closes one gate: its stage is merged and every conformance test passes.

## 7.1 Path to the MVP

The stages E0–E6 stay as specified. The order below reaches a minimal end-to-end compilation
before any optimization: summaries and evidence-based rewrites come only after simple queries
already run correctly on DuckDB.

| Version | Content | Usable result |
|---|---|---|
| 0.1.0 | Contracts (schemas, examples), repository, CI with boundary rules A1–A6 and standards | Contracts can be validated |
| 0.2.0 | E0: load and validate model, catalog and extensions | `semantic-compiler validate` |
| 0.3.0 | **Minimal vertical slice:** E1–E4 and E6 for a single-branch wide-table query (dimensions, base metrics, filters, access and row policies), no E5 rewrites | First correct, secure SQL executed on DuckDB |
| 0.4.0 | E3 complete (paths, reachability, anchors) and E4 stitching of several branches, deduplication | Multi-dataset queries |
| 0.5.0 | SQL form (Layer 2 single-relation subset), additivity and population | Both query forms; balances |
| 0.6.0 | E5: evidence-based rewrites and summary selection | Faster plans, same results |
| 0.7.0 | Lineage, audit record, cache key, `describe`, `explain`, integrity checks | Full outputs; Layer 3 suites block from here (OSS-2.3) |
| 0.8.0 | Context modifiers: period to date, prior period, share of total | Time intelligence |
| 0.9.0 | Python package, CLI complete, performance targets (R1–R3) | Release candidate |
| **1.0.0** | **MVP.** Every gate of 7.1a passes; contracts frozen at major version 1 | Production use |

## 7.1a Gates

Every version closes only when its gates pass in CI; 1.0.0 needs all of them.

| Gate | Verifiable criterion |
|---|---|
| Conformance | The specification files of the version are added to the coverage scope (CS-2.6) and every rule ID of the scope has a passing case or check (T1); Apache Ossie suites match `expected-failures.yaml` (OSS-2.4) |
| Security | Every case of `e2-*` authorization and `e4-*` row-policy rules passes, including denied access, exemption, propagation, policy preservation in summaries, and unauthorized names reported as `E_NAME_NOT_FOUND`; no plan in the suite contains a query or attribute value outside its parameters |
| Determinism | Property tests compile every suite case twice, in separate processes and with shuffled input key order, and get byte-identical plans and hashes |
| Compatibility | Every contract example validates; every schema change bumps its version (A5); documents of the previous minor versions are still accepted |
| Execution | Every `plan` case executes on DuckDB with non-default session settings and returns its expected rows (E6-5.4) |
| Performance | R1–R3 (from 0.9.0) |

## 7.1b What 1.0.0 guarantees

| Guaranteed | Not supported in 1.0.0 (rejected with an error, never answered differently) |
|---|---|
| Every Apache Ossie 0.1.1 model loads (OSS-2.1), with declared deviations DEV-1–DEV-9 | Development-line models outside `ossie_format: development` |
| Wide-table queries with full Layer 3 semantics and its suites passing | Predicates on metric values, subtotals, row listings, per-metric filters |
| SQL form: Layer 2 single-relation subset (E1-4), exact semantics | `JOIN`, `HAVING`, CTEs, subqueries, set operators, window functions in SQL |
| Additivity, population, derived and context metrics; summaries over integer and decimal sums | Window metrics, ranking, top N per group; summaries over floats, holistic or semi-additive metrics |
| Dataset, field and row security, deny by default | Value masking |
| DuckDB SQL, deterministic, session-independent | Other dialects |
| Contracts frozen at major version 1 | — |

## 7.2 After the MVP

| Version | Content |
|---|---|
| 1.1.0 | ClickHouse dialect |
| 1.2.0 | Window metrics (moving windows, running totals), ranking and top N |
| 1.x | Layer 2 SQL growth (F28) |
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
| F1 | Moving windows and ranking (planned for 1.2.0) |
| F2 | Predicates on metric values (`HAVING`) |
| F3 | Subtotals (`ROLLUP`, `CUBE`, `GROUPING SETS`) |
| F4 | Row listings without aggregation |
| F5 | Per-metric filters and filter exemptions |
| F6 | Window functions in model expressions, when Apache Ossie defines them |
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
| F26 | Natural-language translator module (separate repository) |
| F27 | Filling time gaps with an explicit time spine |
| F28 | Layer 2 SQL growth, one construct per release with its suite cases: many-to-one `JOIN`, `HAVING`, CTEs and subqueries, other join types, set operators, until full Layer 2 conformance |
