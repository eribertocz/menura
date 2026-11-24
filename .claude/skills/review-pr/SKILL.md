---
name: review-pr
description: Review an implementation pull request against the specification and post the findings as a review comment. Use when asked to review a pull request.
---

# Review a pull request

GitHub GraphQL is not available in cloud sessions: use the REST API through `gh api` only.

1. Read the pull request, its files and its issue: `gh api repos/{owner}/{repo}/pulls/<number>`, `gh api repos/{owner}/{repo}/pulls/<number>/files`, `gh api repos/{owner}/{repo}/issues/<issue>`.
2. Read only the specification sections the issue lists, plus `AGENTS.md`.
3. Check, and record each finding with file and line:
   - **Traceability:** every rule ID of the issue has at least one passing conformance case; no case claims a rule the code does not implement.
   - **Scope:** only files the issue requires are changed.
   - **Behavior:** no behavior that the specification does not define; every deviation is raised as an issue, not hidden.
   - **Boundaries:** rules A1–A6 hold; no I/O, clock, network or other-module code in the core.
   - **Standards:** chapter 5 rules S, Q and T hold (single responsibility, dependency direction, no `unwrap`, docs, small functions).
   - **Security:** bound parameters only; row policies untouched by later stages.
   - **Determinism:** no iteration over unordered collections into output.
   - **Language:** everything in English (WF-9.1, WF-9.2).
   - **Architecture (S1–S5, S9, S10):** one responsibility per crate and per file, stated in its first doc comment; data types apart from logic; dialect code only in E6; security logic only in `security`; stage representations only in `model` and `representations`; expression text parsed only by `expressions`.
   - **Performance (R1–R3):** no algorithm quadratic in model size; from 0.9.0, the benchmarks meet the targets of §5.6.
   - **Size and names (Q5, Q6):** functions up to 50 lines and files up to 400, or a written justification; names state intent without abbreviations.
   - **Tests (T3, T4):** generated SQL is executed (`rows`); a golden SQL file only where the SQL text is the subject of an E6 rule; property tests where the change touches determinism (P3) or security (P9).
   - **Cases (CS-2.3, CS-2.3a, CS-2.4, CS-2.4a, CS-2.4b, CS-2.4c, CS-2.5, CS-2.7, CS-2.8, CS-2.9, CS-2.10, CS-2.10a, CS-2.10b, CS-2.11, CS-2.12, CS-2.13, CS-2.14, CS-2.15, CS-2.16, CS-2.17, CS-2.18, CS-2.6a, CS-2.6b, CS-4.7, CS-4.8):** output expectations (lineage, explanation codes, audit, `describe`, `integrity_checks`, `explain`) are compared as CS-2.10–CS-2.18 state; a case that needs a load option sets it in `load_options`; golden files regenerated only in a pull request that touches nothing else and says why; row comparison as CS-2.4 states, with the `rows` format of CS-2.4a; expected warnings as CS-2.4b; a `runtime_error` case passes only as CS-2.4c says; runtime-policy cases expect `runtime_error`; the runner reports a part that needs unimplemented stages as pending, never as passed; a case's `location` is compared with the first error's location; a test that covers a rule under CS-2.6a is named after it and tests it, and no implementation pull request edits `spec/coverage-exempt.txt` (CS-2.6b). Differential cases compare result rows with the oracle, and every divergence is classified (CS-4.7).
   - **Apache Ossie suites (CS-3.1, CS-3.2, CS-4.4, CS-4.5, CS-4.6):** suites vendored unchanged at the tracked commit; the commit changes only through the `ossie-update` skill; the runner prepares and compares suite cases as CS-4.4 to CS-4.6 state.
   - **Producer suite (CS-5.1, CS-5.2, CS-5.3, CS-5.4):** each case has its document, its schema document when needed and `expected.yaml`; every catalog code and CAT-8 rule has a case; CI runs them through the public admission functions; the suite stays in this repository.
   - **Principles (P1, P2, P4, P5, P6, P7, P8, P10):** an answer the compiler cannot prove correct is an error (P1); compilation performs no I/O, clock or network access (P2); inputs and outputs use open standards (P4); the plan stays dialect-neutral until E6 (P5); each error is raised at the earliest stage that can detect it (P6); every plan choice is explained (P7); each stage has its defined input and output and is tested alone (P8); Apache Ossie is normative as P10 states.
   - **Apache Ossie tracking (OSS-1.1, OSS-1.2, OSS-3.1, OSS-4.2, OSS-4.3, OSS-4.4, OSS-4.5, OSS-4.6, CS-2.6b):** the model is read at the normative release and the query interfaces at the tracked commit (OSS-1.1); a development-line model is accepted only with `ossie_format: development` (OSS-1.2); extensions use the form Apache Ossie is most likely to adopt (OSS-3.1); release and tracked-commit updates follow OSS-4.2 to OSS-4.6 and the `ossie-update` skill; exemptions in `spec/coverage-exempt.txt` name rules with no observable behavior and give the reason.
   - **Workflow and CI (WF-6.1, WF-6.2, WF-6.6, WF-6.6a, WF-6.7, CI-3.1, CI-3.2):** nothing reaches `main` outside a pull request, no denied command is worked around, the session is a single-repository cloud session, no approval is given; no check is added, removed or weakened outside a specification pull request, and workflows live only in `.github/workflows/`.
   - **Compatibility matrix (CI-4.1, CI-4.2, CI-4.3, CI-4.4, CI-4.5):** a change to `release-assets` or to the matrix generator keeps both assets generated from the tag commit's CI results, one row per channel and platform a job tested, every column of CI-4.3, and the failure of CI-4.5; documentation and release notes claim support only through the matrix.
4. Verify CI is green: `gh api repos/{owner}/{repo}/commits/<head-sha>/check-runs`.
5. Post one comment with `gh api repos/{owner}/{repo}/issues/<number>/comments -X POST -F body=@<file>`, listing findings by severity: blocking, should fix, note. If there are none, say so.
6. Never approve and never merge: approval belongs to the maintainer.
