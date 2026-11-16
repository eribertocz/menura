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
| `conformance-wrappers` | The suite run through the Python package and the CLI, results identical | A4 | Yes |
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

## CI-2 Scheduled

| Check name | Schedule | Action |
|---|---|---|
| `ossie-watch` | Daily | Opens an `ossie-watch` issue for each Apache Ossie release or specification change (OSS-4.1) |

## CI-3 Rules

- **CI-3.1** A check is added, removed or weakened only through a specification pull request (section 6.8).
- **CI-3.2** Workflow files live in `.github/workflows/` and are owned by the maintainer (CODEOWNERS).
