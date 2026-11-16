---
name: review-pr
description: Review an implementation pull request against the specification and post the findings as a review comment. Use when asked to review a pull request.
---

# Review a pull request

1. Read the pull request and its issue: `gh pr view <number>`, `gh issue view <issue>`.
2. Read only the specification sections the issue lists, plus `AGENTS.md`.
3. Check, and record each finding with file and line:
   - **Traceability:** every rule ID of the issue has at least one passing conformance case; no case claims a rule the code does not implement.
   - **Scope:** only files the issue requires are changed.
   - **Behavior:** no behavior that the specification does not define; every deviation is raised as an issue, not hidden.
   - **Boundaries:** rules A1–A6 hold; no I/O, clock, network or other-module code in the core.
   - **Standards:** chapter 5 rules S, Q and T hold (single responsibility, dependency direction, no `unwrap`, docs, small functions).
   - **Security:** bound parameters only; row policies untouched by later stages.
   - **Determinism:** no iteration over unordered collections into output.
   - **Language:** everything in English.
4. Verify CI is green.
5. Post one review comment with `gh pr review <number> --comment`, listing findings by severity: blocking, should fix, note. If there are none, say so.
6. Never approve and never merge: approval belongs to the maintainer.
