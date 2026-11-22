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
| 0.2.0 | E0: load and validate model, catalog and extensions | `menura validate` |
| 0.3.0 | **Minimal vertical slice:** E1–E4 and E6 for a single-branch wide-table query (dimensions, base metrics, filters, access and row policies), no E5 rewrites | First correct, secure SQL executed on DuckDB |
| 0.4.0 | E3 complete (paths, reachability, anchors), joins with deduplication, row-policy propagation, E4 stitching of several branches, derived metrics | Multi-dataset queries |
| 0.5.0 | SQL form (Layer 2 single-relation subset), additivity and population | Both query forms; balances |
| 0.6.0 | E5: evidence-based rewrites and summary selection | Faster plans, same results |
| 0.7.0 | Lineage, audit record, cache key, `describe`, `explain`, integrity checks | Full outputs; Layer 3 suites block from here (OSS-2.3) |
| 0.8.0 | Context modifiers: period to date, prior period, share of total; the model-dependency contract and `menura diff` (F33) | Time intelligence; impact analysis |
| 0.9.0 | Python package, CLI complete (`migrate` included), `menura-mcp` with tool parity (ARC-6.1), performance targets (R1–R3); a smoke test with one external agent or client that uses only public interfaces (discover, compile, a typed error, a refused metric), with a deterministic stub in CI, which never blocks a release | Release candidate |
| **1.0.0** | **MVP.** Every gate of 7.1a passes; contracts frozen at major version 1; a published compatibility matrix per implementation and channel, backed by suite results, never a universal claim | Production use |

## 7.1a Gates

Every version closes only when its gates pass in CI; 1.0.0 needs all of them.

| Gate | Verifiable criterion |
|---|---|
| Conformance | The rules of the version are added to the coverage scope (CS-2.6: whole files, or the rules of its slice before 1.0.0) and every rule ID of the scope has a passing case or check (T1); Apache Ossie suites match `expected-failures.yaml` (OSS-2.4) |
| Security | Every case of `e2-*` authorization and `e4-*` row-policy rules passes, including denied access, exemption, propagation, policy preservation in summaries, and unauthorized names reported as `E_NAME_NOT_FOUND`; no plan in the suite contains a query or attribute value outside its parameters |
| Determinism | Property tests compile every suite case twice, in separate processes and with shuffled input key order, and get byte-identical plans and hashes |
| Compatibility | Every contract example validates; every schema change bumps its version (A5); documents of the previous minor versions are still accepted |
| Execution | Every `plan` case executes on DuckDB with non-default session settings and returns its expected rows (E6-5.4) |
| Performance | R1–R3 (from 0.9.0) |

## 7.1c Version slices before 1.0.0

Each version before 1.0.0 compiles only the queries of its slice. A query outside the slice is
rejected, never answered partially or without a rule it needs (above all a row policy).

| ID | Rule |
|---|---|
| VS-1 | **0.3.0 slice.** A wide-table query whose metric components, dimensions, filters and `time_grains`, `date_range` and `order_by` fields all belong to **one dataset** (its home dataset), with no `paths`; metrics without the derived, additivity, population or context extensions; dimension-only queries on one dataset; the home dataset's access, field restrictions and its own row policies. A home dataset that reaches, through many-to-one steps (E4-6.5), another dataset that declares `row_access` needs propagation, so it is outside the slice. This is decided from the model alone, whatever the user's roles, so that the answer never depends on policies the user does not hold. |
| VS-1b | **0.4.0 slice.** Every wide-table query is in the slice, with these exceptions: an element that needs additivity or population (0.5.0) or a context metric (0.8.0); the SQL form, detected as VS-2 states; and `compile` with `audit: true`, which returns `E_NOT_YET_SUPPORTED` before every other check (it wins over every row of E2-5), with `location` `null` because the option is not part of an input document, until VS-1e adds it. Several datasets, `paths`, derived metrics and row-policy propagation are in the slice. |
| VS-1c | **0.5.0 slice.** Every query is in the slice, in either form, with two exceptions: an element that needs a context metric (0.8.0), and `compile` with `audit: true`, rejected as VS-1b states until VS-1e adds it. The SQL form within the E1-4 subset is in the slice, and so are metrics with an `additivity` declaration (`non_additive` or `semi_additive`) and the populations they read. A SQL construct outside E1-4 is `EQRY-05`, never `E_NOT_YET_SUPPORTED`. |
| VS-1d | **0.6.0 slice.** The slice of VS-1c, unchanged: E5 changes plans, never which queries compile or their results. Without a catalog state, E5 applies only the rewrites that need no evidence (E5-2.1: E5-3.1, E5-3.4, E5-3.6); with one, also E5-3.2, E5-3.3 and E5-3.5 (E5-4). |
| VS-1e | **0.7.0 slice.** The slice of VS-1d, with every output of the compiler: `compile` with `audit: true` (PLN-6), the `lineage` (PLN-4) and `explanation` (PLN-1) of every compiled plan, and the functions `describe` (DSC), `explain` and `integrity_checks` (CHK) of architecture section 3.5 are in the slice. The only exception left is an element that needs a context metric (0.8.0). `compile` with `audit: true` no longer returns `E_NOT_YET_SUPPORTED`: it returns the audit record with the compiled plan or with the compile error, and the plan and the error are those of the same compilation with `audit: false`. |
| VS-2 | Outside the slice, `compile` returns `E_NOT_YET_SUPPORTED`, naming the capability and the version that adds it (table of 7.1), chosen and located as E2-5.4 states. Before 0.5.0, the SQL form is detected in E1, right after the JSON Schema check of E2-5 row 1 (`E_INVALID_QUERY`) and before `EQRY-05` and `EQRY-13`, so no SQL text is parsed before the version that adds the SQL form. Every other capability is detected after every check of E2 has passed (after E2-5 row 12b), so a query reports the same errors as it will in the version that supports it, up to that point. |
| VS-3 | Each later version widens the slice to the content of its row in 7.1. `E_NOT_YET_SUPPORTED` disappears at 1.0.0; what 1.0.0 does not support is rejected with the codes of 7.1b's table. |
| VS-4 | The conformance runner reports the query part of a case whose compilation returns `E_NOT_YET_SUPPORTED` as pending, never as passed or failed (CS-2.8), unless the case itself expects `error: E_NOT_YET_SUPPORTED`: such a case tests the slice boundary and is compared like any `error` case. |
| VS-5 | **Superseded by VS-1e in 0.7.0.** No version from 0.7.0 on applies this rule; its ID is kept so that earlier references resolve. Historical note, for 0.3.0 to 0.6.0: a compiled plan carried `producer`, `dialect`, `data_source`, `sql`, `parameters`, `result_schema`, `limits`, `warnings` and `cache_key` as PLN-1–PLN-3 state (`data_versions` was `null` without a catalog state). `lineage` and `explanation` were empty lists. `compile` with `audit: true` returned `E_NOT_YET_SUPPORTED`, checked before every other check (it wins over every row of E2-5), with `location` `null` because the option is not part of an input document. `describe`, `explain` and `integrity_checks` were added in 0.7.0. |

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
| F21 | Done for the development line: dataset-scoped metrics (QRY-3.2), and `AGG(dataset.metric)` in derived metrics from 0.4.0 (DER-2.2, DER-5.1.2) |
| F22 | Project-specific `vendor_name` replacing `COMMON` + `namespace` |
| F23 | Apache Ossie Layer 4 (ontology) |
| F24 | Done: the project is named Menura (`menura`); the former name is accepted until 1.0.0 (CON-0.5) |
| F26 | Natural-language translator module (separate repository) |
| F27 | Filling time gaps with an explicit time spine |
| F28 | Layer 2 SQL growth, one construct per release with its suite cases: many-to-one `JOIN`, `HAVING`, CTEs and subqueries, other join types, set operators, until full Layer 2 conformance |
| F29 | Scheduled for 0.9.0: `menura-mcp` (ARC-6.1, ARC-6.2, decision 0007) |
| F30 | An in-toto Statement v1 export of the audit record (PLN-6) and the plan, built by an external adapter, never by the core (after 0.7.0) |
| F31 | A limited Kani pilot on small pure functions, with stated properties and limits; never presented as a global proof |
| F32 | A `wasm32` build profile study, never required |
| F33 | Scheduled for 0.8.0: `menura diff`, a pure CLI command that classifies changes to metrics and fields between two models (added, changed, removed) from the per-definition hashes and `migrate` (E0H-3), with the model-dependency contract (decision 0007) |
| F34 | Context modifiers (`prior_period`, `period_to_date`) in the typed query, after 1.0.0; until then they are model metrics (CTX) |
