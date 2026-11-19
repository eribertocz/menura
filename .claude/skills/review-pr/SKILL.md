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
   - **Cases (CS-2.3, CS-2.4, CS-2.5, CS-2.7, CS-2.8, CS-2.9):** a case that needs a load option sets it in `load_options`; golden files regenerated only in a pull request that touches nothing else and says why; row comparison as CS-2.4 states; runtime-policy cases expect `runtime_error`; the runner reports a part that needs unimplemented stages as pending, never as passed; a case's `location` is compared with the first error's location.
   - **Apache Ossie suites (CS-3.1, CS-3.2, CS-4.4, CS-4.5, CS-4.6):** suites vendored unchanged at the tracked commit; the commit changes only through the `ossie-update` skill; the runner prepares and compares suite cases as CS-4.4 to CS-4.6 state.
   - **Producer suite (CS-5.1, CS-5.2, CS-5.3, CS-5.4):** each case has its document, its schema document when needed and `expected.yaml`; every catalog code and CAT-8 rule has a case; CI runs them through the public admission functions; the suite stays in this repository.
   - **Workflow and CI (WF-6.1, WF-6.2, WF-6.6, WF-6.7, CI-3.1, CI-3.2):** nothing reaches `main` outside a pull request, no denied command is worked around, the session is a single-repository cloud session, no approval is given; no check is added, removed or weakened outside a specification pull request, and workflows live only in `.github/workflows/`.
4. Verify CI is green: `gh api repos/{owner}/{repo}/commits/<head-sha>/check-runs`.
5. Post one comment with `gh api repos/{owner}/{repo}/issues/<number>/comments -X POST -F body=@<file>`, listing findings by severity: blocking, should fix, note. If there are none, say so.
6. Never approve and never merge: approval belongs to the maintainer.
