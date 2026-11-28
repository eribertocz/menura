# Menura — Specification

> Status: Draft 0.1.0 · License: Apache 2.0 · Project name is provisional.

Menura (`menura`, formerly `semantic_compiler`) turns a question asked in business terms (metrics, dimensions, filters) into
correct, secure, deterministic SQL for a target database. It speaks standard SQL, reads standard
storage, and uses **Apache Ossie** as its semantic model standard.

## Reading order

| # | Document | Scope |
|---|---|---|
| — | [INDEX.md](INDEX.md) | Every rule ID and section, with its file. **Start here to find anything.** |
| 1 | [Vision](01-vision.md) | What the compiler is, and what it is not |
| 2 | [Principles](02-principles.md) | Non-negotiable rules (P1–P10) |
| 3 | [Architecture](architecture/README.md) | Modules, contracts, delivery, integration, boundaries, lineage and audit |
| 4 | [Apache Ossie conformance](04-ossie-conformance.md) | Normative base, extensions lifecycle, release tracking |
| 5 | [Engineering standards](05-engineering-standards.md) | Responsibility, dependencies, quality, tests, performance |
| 6 | [Development workflow](06-development-workflow.md) | Roles, issues, pull requests, review, language |
| 7 | [Roadmap](07-roadmap.md) | Versions to the MVP (1.0.0) and after |
| 8 | [Conformance suite](08-conformance-suite.md) | Case format and layout |
| 9 | [Continuous integration](09-continuous-integration.md) | Every CI check, its trigger and whether it blocks |
| — | [Contracts](contracts/README.md) | Every document that crosses a module boundary |
| — | [Stages](stages/README.md) | E0–E6 |
| — | [Extensions](extensions/README.md) | Additivity, population, derived, context, security, summaries |
| — | [Glossary](glossary.md) | Terms |

## Conventions

- The key words MUST, MUST NOT, SHOULD, SHOULD NOT and MAY are used as described in RFC 2119 and RFC 8174.
- Every identifier, error code, file and field name is in English.
- Normative rules have IDs (`CAT-7.2`, `E3-3.5`, …). Item *n* of a numbered list under rule `X` is `X.n` (for example `CAT-7.2.4`).
- **Behavioral rules** (prefixes of stages, contracts and extensions: `E0…E6`, `STG`, `CAT`, `QRY`, `SEC`, `PLN`, `DSC`, `CHK`, `MDL`, `CON`, `ADD`, `POP`, `DER`, `CTX`, `SCY`, `SUM`, `EXT`, `OSS-2`, `OSS-5`, `DEV`, `ARC-5`, `ARC-6`, `PLT`, `VAL`, `MIG`, `DEP`) each need at least one conformance case or test (CS-2.6a), unless CS-2.6b exempts them. **Process rules** (`P`, `A`, `S`, `Q`, `T`, `R`, `WF`, `CS`, `CI`, `OSS-1`, `OSS-3`, `OSS-4`) each need a named CI check or a review item in the `review-pr` skill. Backlog items (`F`) are not rules.
- Codes: `E` + area + `-` + number for errors (`E0A-01`, `E2-01`, `EQRY-05`), `W` for warnings, `R` + area for runtime failures of the generated SQL (`RADD-01`). Apache Ossie codes (`E_NO_PATH`, …) are kept verbatim.
