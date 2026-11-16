# Agent instructions

These rules are mandatory. If a rule conflicts with an issue, the rule wins: stop and report.

## 0. Language

English only, everywhere in this repository: code, identifiers, comments, documentation,
commit messages, branch names, issues, pull requests, review comments and error messages.
This holds even when the person you are talking to writes in another language.

## 1. Your role

- **Implementation session:** implement exactly one issue. Branch, commit, push, open a pull request.
- **Specification session:** edit only the specification scope (`spec/06-development-workflow.md` §6.6), on `spec/<topic>` branches, create issues, review pull requests.
- Never merge, approve, tag, release or change repository settings. These commands are denied by `.claude/settings.json`; never try to work around a denied command.

## 2. What to read (token budget)

1. Read the issue.
2. Open `spec/INDEX.md` only to locate a rule ID or section you were not given.
3. Read **only** the files and sections the issue lists under "Read", plus `spec/08-conformance-suite.md` CS-1 and CS-2 when writing cases.
4. Do not read other specification files, `spec/decisions/`, or other modules' code.

If the listed sections are not enough to implement the issue without guessing, that is a
specification defect: open an issue labeled `spec-defect` and stop.

## 3. Never

- Never invent behavior the specification does not define.
- Never change the specification scope (`spec/`, `AGENTS.md`, `CLAUDE.md`, `.claude/`, issue and pull request templates, `scripts/build_index.py`, `scripts/check_examples.py`) from an implementation session.
- Never weaken a check: no disabled lints, skipped tests, `#[allow]` without a written reason, or widened allowlists.
- Never import code from another module; cross-module data is a contract document only.
- Never add I/O, network, clock or database access to the core.
- Never concatenate values into SQL; use bound parameters.
- Never remove, bypass or reorder security filters.
- Never use `unsafe` anywhere, and never `unwrap`, `expect` or `panic!` in library code (tests may use them).
- Never commit secrets, generated artifacts, or commented-out code.
- Never touch files outside the issue's scope.

## 4. Engineering principles

**Architecture**
- One responsibility per crate, module, file and function. If a description needs "and", split it.
- Dependencies point one way: `contracts ← model ← security ← e0…e5 ← generator-duckdb ← core ← wrappers`. No cycles.
- Each stage depends only on the representation it consumes, never on another stage's internals.
- Data types are separate from the logic that transforms them.
- Keep the core pure and deterministic: the same input always yields the same output, byte for byte.
- Depend on stable abstractions at boundaries; keep implementation details private (`pub(crate)`).

**Code**
- Prefer explicit, simple code over clever code. Readability first, then performance.
- Names state intent in full English words. No abbreviations, no Hungarian notation.
- Functions are small (target ≤ 50 lines) and do one thing; files target ≤ 400 lines. Exceeding a target needs a written justification in the pull request (Q5).
- Make invalid states unrepresentable: use enums and newtypes rather than strings and flags.
- Every failure is a typed error carrying its specification code and source location.
- No global mutable state. No hidden side effects.
- Every public item is documented: what it does, its errors, and an example where useful.

**Maintainability and scalability**
- Change one thing per pull request. Refactor and behavior change never share a commit.
- No premature abstraction: add an interface when the second real implementation exists.
- Avoid quadratic algorithms over model size; respect the performance targets in `spec/05-engineering-standards.md` §5.6.
- Prefer immutable data and pure transformations; share by reference, not by copy.
- Dependencies: add a crate only if it is maintained, widely used, license-compatible, and approved in `deny.toml` by the maintainer.

**Testing**
- Write the tests for the issue's rule IDs first, see them fail, then implement.
- Every rule ID in the issue has at least one passing conformance test named after it.
- Generated SQL is checked against its golden file and by execution on DuckDB.
- A bug fix includes a test that fails without the fix.

## 5. Workflow

1. Branch: `issue-<number>-<short-name>` (implementation) or `spec/<topic>` (specification). Any other name fails CI.
2. Commits: Conventional Commits with the issue number (`feat(e0): validate relationship keys (#12)`).
3. Before pushing, run: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check`, `just conformance`, `python3 scripts/build_index.py --check`, `python3 scripts/check_examples.py`.
4. Open the pull request with the template. Fill in every section, including the rule IDs covered.
5. Respond to review by pushing new commits. Do not force-push after review has started.

## 6. When in doubt

Stop and open an issue. A question costs minutes; a wrong guess costs a rewrite.
