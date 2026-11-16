---
name: implement-issue
description: Implement one GitHub issue of this repository end to end, from reading the issue to opening the pull request. Use when asked to implement, work on or fix an issue.
---

# Implement an issue

GitHub GraphQL is not available in cloud sessions: use the REST API through `gh api` only.

1. Read the issue with `gh api repos/{owner}/{repo}/issues/<number>`. Confirm it has: stage, rule IDs, "Read" list, acceptance criteria, out of scope, dependencies. If any is missing, open a `spec-defect` issue naming what is missing, and stop.
2. Check that every issue listed under "Depends on" is closed. If not, stop and say which one is open.
3. Read only the files and sections under "Read". Use `spec/INDEX.md` only to locate a rule ID you were not given. If the sections are not enough to implement without guessing, open a `spec-defect` issue and stop.
4. Create the branch: `git switch -c issue-<number>-<short-name>` from an up-to-date `main`.
5. For each rule ID, write its conformance cases first (use the `add-conformance-case` skill; reading `spec/08-conformance-suite.md` CS-1 and CS-2 is always allowed), run them, and confirm they fail.
6. Implement the smallest code that makes them pass, following `AGENTS.md` section 4.
7. Run every check: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check`, `just conformance`, `python3 scripts/build_index.py --check`, `python3 scripts/check_examples.py`.
8. Review your own diff: only files in scope, no commented-out code, no `unwrap`, every public item documented.
9. Commit with Conventional Commits and the issue number. Push the branch.
10. Open the pull request with `gh api repos/{owner}/{repo}/pulls -X POST -f title=... -f head=<branch> -f base=main -F body=@<file>`, filling every section of the template, including the rule IDs and the conformance cases that cover each. Then add the `impl` label with `gh api repos/{owner}/{repo}/issues/<pr>/labels -X POST -f "labels[]=impl"`.
11. Stop. Do not merge.
