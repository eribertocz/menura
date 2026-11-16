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
contracts  ←  model  ←  security  ←  e0 … e5  ←  generator-duckdb (e6)  ←  core  ←  wrappers (python, cli, http)
```

`security` holds access and row-policy logic (S5) and is used by E0, E2, E4 and E5. `core` orchestrates the stages and calls the generator for plans and integrity checks.

| ID | Rule | Check |
|---|---|---|
| S6 | Dependencies point in one direction only; cycles are forbidden. | Workspace dependency graph test |
| S7 | A stage depends only on the intermediate representation it consumes, never on another stage's internals. | Crate visibility (`pub(crate)`) and dependency allowlist |
| S8 | Wrappers depend only on `core`. | Dependency allowlist |

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
| T1 | Every behavioral rule ID has at least one conformance case, and every process rule ID a named CI check or review item (README conventions); CI fails otherwise. |
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
