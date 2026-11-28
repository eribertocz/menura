# 9. Continuous integration

Every check that CI runs, with its trigger, the rule it enforces and whether it blocks. CI runs on
GitHub Actions (WF-8.0). Blocking checks are required by branch protection.

## CI-1 On every pull request and every push to `main`

| Check name | Command or mechanism | Enforces | Blocks |
|---|---|---|---|
| `format` | `cargo fmt --check` | Q3 | Yes |
| `lint` | `cargo clippy --all-targets -- -D warnings` with `pedantic`, `unwrap_used`, `expect_used`, `panic`, `missing_docs` denied | Q2, Q3, Q4, Q7 | Yes |
| `unsafe` | Every crate root has `#![forbid(unsafe_code)]` | Q1 | Yes |
| `test` | `cargo test --all-targets` | T2, T5 | Yes |
| `dependencies` | `cargo deny check` (allowlist, banned I/O crates, licenses) | A1, A2 | Yes |
| `dependency-graph` | Workspace graph test: direction of chapter 5.2, no cycles, wrappers depend only on `core` | S6–S8, A4 | Yes |
| `conformance` | `just conformance` with non-default DuckDB settings | T1–T4, E6-5.4 | Yes |
| `conformance-wrappers` | The suite run through the Python package, the CLI and the MCP server, results identical to `core` (ARC-6.9) | A4 | Yes |
| `conformance-clickhouse` | From 1.1.0: every `plan` and `runtime_error` case run with `dialect: clickhouse` on the pinned ClickHouse release, with the E6-7.8 settings reversed at session level and a server time zone that is not UTC (E6-7.10, CS-2.3a) | T3, E6-7.10 | Yes |
| `platforms` | On macOS arm64, macOS x86_64 and Windows x86_64: build and test the crates `core`, `cli` and `mcp`; compile every conformance case with `core` and compare each output with its golden file as CS-2.3 and CS-2.10 state, without executing SQL and with no pending case; prove that every CLI and MCP output equals the `core` output for the same call (ARC-6.9); and check that the SHA-256 of every CLI output equals the one the `conformance-wrappers` job computes on Linux x86_64 for the same call (PLT-1.2) | PLT-1 | Yes |
| `agent-smoke` | From 0.9.0: one scripted client that uses only `menura-mcp` (discover with `describe`, `compile`, a typed error, a refused metric), against a deterministic stub; never blocks a release (7.1) | ARC-6.7 | No |
| `ossie-suites` | Apache Ossie suites at the tracked commit against `expected-failures.yaml` | OSS-2.3, OSS-2.4 | Layer 3 from 0.7.0; Layer 2 never until it blocks (OSS-2.3) |
| `rule-coverage` | Every behavioral rule ID of the coverage scope (CS-2.6) is cited by a case, and every process rule ID of the scope by a CI check or review item; the rest is reported | T1, CS-2.6 | Yes |
| `contracts` | Each contract schema is valid; a changed schema has a bumped version and updated examples | A3, A5 | Yes |
| `examples` | `python3 scripts/check_examples.py` | Contracts chapter | Yes |
| `index` | `python3 scripts/build_index.py --check` | Index | Yes |
| `stubs` | End-to-end tests with stub modules that only speak the contracts | A6 | Yes |
| `benchmarks` | Benchmarks of R1–R3; a regression above 10 % fails | R1–R3 | Yes |
| `scope` | Branch name prefix vs files changed (`spec/` inside the specification scope only, `issue-` outside it only) | WF-6.3 | Yes |
| `co-author` | Every commit has a `Co-Authored-By` trailer | WF-6.4 | Yes |
| `issue-label` | The linked issue carries exactly one of `spec` or `impl` | WF-6.5 | Yes |
| `language` | Non-English titles, bodies and commit subjects | WF-9.3 | No (warning) |

**Agent smoke test.** From 0.9.0, the `agent-smoke` job runs a fixed client script, with no language model, against `menura-mcp` over standard input and output, on a conformance model: `initialize` and `tools/list`; `describe`; `compile` of a valid query (a plan); `compile` with a misspelled metric (`E_NAME_NOT_FOUND` with `alternatives`, PLN-5.4); and `compile` of a metric the context may not use (the same error as a missing metric, PLN-5.4a). It uses only public interfaces, and its result never blocks a release (7.1).

## CI-2 Scheduled

| Check name | Schedule | Action |
|---|---|---|
| `ossie-watch` | Daily | Opens an `ossie-watch` issue for each Apache Ossie release or specification change (OSS-4.1) |
| `release-assets` | When a release is published, or dispatched by hand with a tag | Attaches every `spec/contracts/schemas/*.schema.json` of the release's tag as a release asset, replacing an asset of the same name, then fails if any schema of the tag is missing from the release (CON-0.3); from 1.0.0 also attaches the compatibility matrix (CI-4) |

## CI-3 Rules

- **CI-3.1** A check is added, removed or weakened only through a specification pull request (section 6.8).
- **CI-3.2** Workflow files live in `.github/workflows/` and are owned by the maintainer (CODEOWNERS).

## CI-4 Compatibility matrix

From 1.0.0 every release publishes a compatibility matrix (roadmap 7.1, row 1.0.0). It states only what CI verified on the release's tag commit, never a universal claim.

- **CI-4.1** **Assets.** The `release-assets` workflow attaches `compatibility-matrix.json` and `compatibility-matrix.md` to the release. It generates both from the results of the CI runs of the tag commit; neither is written by hand. The Markdown file is a table rendering of the JSON file and adds nothing.
- **CI-4.2** **Rows.** One row per channel and platform that CI built and tested on the tag commit: `core` (the Rust crate's public functions), `cli` (the `menura` binary) and `mcp` (the `menura-mcp` binary) on the platform of the `conformance` and `conformance-wrappers` jobs, and `python` once per wheel platform of ARC-6.3 whose `python-wheels` job installed the wheel and ran the package tests. They also appear, for `core`, `cli` and `mcp`, once per platform of the `platforms` job that passed (PLT-1.1). These rows are for the dialect `duckdb`. For a tag whose version is 1.1.0 or later by SemVer precedence (so a pre-release of 1.1.0 is earlier), `core` also has a row for the dialect `clickhouse` on the platform of the `conformance-clickhouse` job (E6-7.10); an earlier tag has no such row and does not depend on that job. A channel, platform or dialect that no job tested has no row.
- **CI-4.3** **Columns.** Each row carries the Menura version and the commit; the channel and the platform (operating system and architecture); the dialect and the exact engine version the suite ran on (the DuckDB version with the time-zone data version of its `icu` extension, or the ClickHouse server version with the version of its time-zone data, E6-7.16, EXE-6.1); the Apache Ossie normative version and tracked commit of the capability profile (DSC-4); the conformance cases passed and in total; the Apache Ossie suite outcomes as passed, expected failures and unexpected results (OSS-2.4), or `null` on a row whose dialect the `ossie-suites` job does not run (the suites run on DuckDB only), never the outcomes of another dialect; for `cli`, `mcp` and `python`, whether `conformance-wrappers` (or, on a wheel platform without it, the package tests) passed; on a row of the `platforms` job, the engine version and the Apache Ossie suite outcomes are `null`, because no SQL runs there, the conformance counts are those of that job's own compile-only run, and the wrapper column states whether that job passed; and the URL of each CI run it reports.
- **CI-4.4** **Claims.** Support is claimed only for the rows of the matrix. Documentation and release notes cite the matrix instead of restating platforms or versions, and a combination it does not list is unverified, not supported.
- **CI-4.5** **Failure.** The workflow fails, and attaches no matrix, if any job a row depends on did not pass on the tag commit or did not run on it.
