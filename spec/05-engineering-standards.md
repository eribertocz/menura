# 5. Engineering standards

These rules are normative for the reference implementation. They are checked by CI wherever a
tool can check them; the rest are checked in review against this chapter.

## 5.1 Single responsibility

| ID | Rule |
|---|---|
| S1 | Each crate has one responsibility. Each stage (E0–E6) is its own crate. |
| S2 | Each module and each file has one responsibility, stated in its first doc comment. A file whose description needs "and" MUST be split. |
| S3 | Data types (plan, model, catalog) live apart from the logic that transforms them. |
| S4 | Dialect-specific code exists only in generator crates (E6). No other crate may mention a dialect. |
| S5 | Security logic exists only in the security crate. Other crates call it and never re-implement it. |

## 5.2 Dependency direction

```
contracts  ←  model  ←  expressions  ←  representations  ←  security  ←  e0 … e5  ←  generator-duckdb (e6)  ←  core  ←  wrappers (python, cli, http)
```

`expressions` holds the expression parser and its normalization (S10), used by E0, E1 and `security`. `representations` holds the data passed between stages (S9) and depends on `model`, because a bound query and a plan refer to model elements. `security` holds access and row-policy logic (S5) and is used by E0, E2, E4 and E5. `core` orchestrates the stages and calls the generator for plans and integrity checks. The test-tooling crate `conformance-runner` (ARC-5.3) depends on `core` and `contracts`; nothing depends on it.

| ID | Rule | Check |
|---|---|---|
| S6 | Dependencies point in one direction only; cycles are forbidden. | Workspace dependency graph test |
| S7 | A stage depends only on the intermediate representation it consumes, never on another stage's internals. | Crate visibility (`pub(crate)`) and dependency allowlist |
| S8 | Wrappers depend only on `core`. | Dependency allowlist |
| S9 | Every representation passed between stages is a data type in a shared crate, never in a stage crate: the internal model (E0H) in `model`; the query tree (E1), the bound query (E2), the paths (E3) and the logical plan (E4, E5 and the integrity checks of CHK-4.2) in `representations`. These crates hold data and the constructors that keep it valid, never stage logic (S3). A stage crate exports only its stage function and its error type. | Workspace dependency graph test plus review |
| S10 | Expression text (E0F-2) is parsed and normalized (E0H-1.3) in one place, the crate `expressions`, which produces the expression tree type of `model`. Every stage and `security` that reads expression text calls it; none has its own parser. | Workspace dependency graph test plus review |

## 5.3 Code quality

| ID | Rule | Check |
|---|---|---|
| Q1 | `unsafe` is forbidden in every crate. | `#![forbid(unsafe_code)]` in every crate root |
| Q2 | No `unwrap`, `expect` or `panic!` in library code; every failure is a typed error. | Clippy lints set to deny |
| Q3 | Formatting and lints are clean: `rustfmt`, and `clippy` with `pedantic` lints, warnings denied. | CI |
| Q4 | Every public item has documentation. | `missing_docs` set to deny |
| Q5 | Functions stay small and do one thing. Targets: 50 lines per function, 400 lines per file. Exceeding a target requires a written justification. | Lint plus review |
| Q6 | Names describe intent, and abbreviations are avoided. | Review |
| Q7 | No dead code and no commented-out code. | Lints plus review |

## 5.4 Testing

| ID | Rule |
|---|---|
| T1 | Every behavioral rule ID has at least one conformance case, and every process rule ID a named CI check or review item (README conventions). CI fails when a rule ID of the coverage scope (CS-2.6) is not covered; other rule IDs are reported only. |
| T2 | Unit tests live next to the code; conformance tests live in the shared suite. |
| T3 | Generated SQL is always executed on the reference engine (`rows`). A golden SQL file is added when the SQL text itself is the subject of a rule (E6), and is then compared byte for byte. |
| T4 | Property-based tests cover the determinism (P3) and security (P9) principles. |
| T5 | Every bug fix adds a test that fails without the fix. |

## 5.5 Decisions and changes

- Architectural decisions are recorded as numbered decision records in the repository.
- A behavior that this specification does not define is a specification defect. It is reported and decided; it is never invented in code.
- A stage is done only when all of its conformance tests pass.

## 5.6 Performance targets

| ID | Target | Check |
|---|---|---|
| R1 | `load` handles models of up to 500 datasets and 5,000 fields. | Benchmark in CI |
| R2 | `compile` takes under 10 ms at the 99th percentile on the reference benchmark model. | Benchmark in CI; a regression above 10 % fails the build |
| R3 | Compilation time grows at most linearly with the number of query items. | Benchmark sweep in CI |

- **R4** **Benchmarks.** The crate `benchmarks` holds the benchmarks. Like `conformance-runner` it is test tooling, is never shipped and calls only the public functions of `core`. Its models and queries are generated by code in the crate from fixed parameters, so every run reads the same inputs.
- **R5** **Reference benchmark model.** 50 datasets in a star and a chain of many-to-one relationships, 500 fields, 100 metrics (base, derived, semi-additive and context metrics), row policies on 10 datasets with propagation, and two summaries; with it, 20 reference queries covering the slice (VS-1f) and one security context. The R1 model has 500 datasets and 5,000 fields of the same kinds.
- **R6** **Measures.** Each query is compiled 20 times to warm up, then 200 times; its time is the 99th percentile of those wall-clock times. R1 passes when `load` of the R1 model succeeds in under 2 s. R2 passes when every reference query is under 10 ms. R3 compiles a query with 1, 2, 4, …, 64 dimensions and metrics, taken from a fixed set of datasets reached by the same joins, so that only the number of items grows, and passes when each doubling at most doubles the cost plus 10 %.
- **R7** **Regression.** The regression check of R2 and the sweep of R3 count instructions (Valgrind Callgrind), not wall-clock time, so they do not depend on the CI runner's load. The counts of the last release are committed in `benchmarks/baseline.json`; a count more than 10 % above its baseline fails the build. Only a pull request that names the cause in its description may update the baseline.
