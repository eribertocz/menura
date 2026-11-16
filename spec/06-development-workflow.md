# 6. Development workflow

## 6.1 Roles

| Role | Who | Responsibilities |
|---|---|---|
| Maintainer | Project owner (a human) | Approves the specification, reviews and merges pull requests, creates release tags. The final authority. |
| Specification author | Claude, specification session | Writes and corrects the specification. Turns each specification task into an issue. Never writes implementation code. |
| Implementer | Claude Code, implementation session | Implements one issue at a time: branch, commits, push, pull request. Never merges, never tags, never edits the specification. |
| Ossie watcher | Scheduled CI job | Opens an issue when Apache Ossie publishes a release or changes its specification. |

## 6.2 Who does what

| Action | Maintainer | Specification author | Implementer | Watcher |
|---|---|---|---|---|
| Edit specification | Approves | Yes, by pull request | No; reports defects as issues | No |
| Create issues | Yes | Yes, from specification tasks | Only to report a specification defect or a bug it found | Yes, Apache Ossie changes |
| Create branch, commit, push | — | Specification branches only | Yes, one branch per issue | No |
| Open pull request | — | Specification pull requests | Yes | No |
| Review pull request | Yes, required | — | Responds to review | — |
| Merge to `main` | **Only the maintainer** (by hand, in GitHub) | No (denied by WF-6.2) | No (denied by WF-6.2) | No |
| Create tag and release | **Only the maintainer** | No (denied) | No (denied) | No |

## 6.3 Repository rules

- `main` is protected by a ruleset: no direct pushes, pull request required, blocking CI checks required (WF-6.1).
- Tags are protected: only the maintainer can create them.
- Branch names: `issue-<number>-<short-name>` (implementation) and `spec/<topic>` (specification), WF-6.3.
- Commits follow Conventional Commits (`feat:`, `fix:`, `test:`, `docs:`, `refactor:`), with the issue number in the message.
- One issue produces one pull request. A pull request touches only what its issue asks for.

## 6.4 Issue format

Every implementation issue contains:

1. The stage and the specification section it implements.
2. The rule IDs it covers.
3. **Read:** the exact files and sections to read, and nothing else.
4. Executable acceptance criteria: the conformance cases that must pass.
5. What is explicitly out of scope.
6. **Depends on** and **Parallel with**: the issues it needs merged first, and those it can run alongside.

## 6.5 Pull request checklist

A pull request can be merged only if:

- every rule ID in its issue has passing tests,
- CI is green, including the boundary rules (A1–A6) and the engineering standards (chapter 5),
- it changes no file outside its issue's scope,
- every deviation from the specification has been raised as an issue and is not hidden in code.

## 6.6 Two sessions, one GitHub identity

The specification session and the implementation session are two separate Claude sessions with
different instructions. Both act on GitHub through the **maintainer's account**; GitHub cannot
tell them apart. The separation is therefore enforced by mechanisms that do not depend on the
agent's goodwill:

| ID | Mechanism | Enforced by |
|---|---|---|
| WF-6.1 | **No direct writes to `main`.** A ruleset on `main` requires a pull request and the blocking CI checks, and forbids force pushes and deletion. Tags are protected by a second ruleset. | GitHub |
| WF-6.2 | **Only the maintainer merges, tags and changes settings.** The committed `.claude/settings.json` denies, at tool level, the commands that merge, approve, tag, release, push to `main`, call the raw GitHub API or change repository settings. Deny rules apply as soon as a session starts. | Claude Code |
| WF-6.3 | **Scope by branch.** Specification sessions push only branches named `spec/<topic>`; implementation sessions only `issue-<number>-<topic>`. The `scope` CI check fails a `spec/` branch that changes files outside the specification scope, an `issue-` branch that changes files inside it, and any other branch name. | GitHub Actions |
| WF-6.4 | **History shows the AI.** Every commit carries a `Co-Authored-By` trailer naming the AI. | Commit convention, checked by CI |
| WF-6.5 | **One session per issue.** Each issue carries exactly one of the labels `spec` or `impl`; other labels (`spec-defect`, `ossie-watch`, `study`) describe it and never assign it. | `issue-label` CI check |

The **specification scope** is `spec/`, `AGENTS.md`, `CLAUDE.md`, `.claude/`, `.github/ISSUE_TEMPLATE/`,
`.github/PULL_REQUEST_TEMPLATE.md`, `scripts/build_index.py` and `scripts/check_examples.py`.

- **WF-6.6** Sessions MUST run as single-repository cloud sessions started in this repository, because only those read the committed `.claude/settings.json`.
- **WF-6.7** Approval is the maintainer's merge itself: GitHub does not let an account approve its own pull request, so no approval count is required. The maintainer reviews the pull request, and the specification session's review comment (section 6.7), before merging.

## 6.7 Pull request review

Every implementation pull request is reviewed twice:

1. **Specification review** by the specification session. It checks traceability (every rule ID
   tested), scope (only the issue's files), engineering standards and any deviation from the
   specification, and posts its findings as a review comment. It never approves and never merges.
2. **Maintainer review**, which is the approval that branch protection requires, followed by the merge.

## 6.8 Who owns the CI rules

- **WF-8.0** CI runs on **GitHub Actions**, automatically: on every push to a pull request branch, on every merge to `main`, and on a daily schedule (Apache Ossie watch, OSS-4.1). Branch protection makes the blocking checks required, so a pull request cannot be merged until they pass.


| Layer | Defined by | Implemented by | Changed by |
|---|---|---|---|
| What CI must check (rules A1–A6, S, Q, T) | This specification | — | Specification pull request, approved by the maintainer |
| Workflow files (`.github/workflows/`, `deny.toml`, lint configuration) | This specification | Implementer, in the 0.1.0 issues | Implementation pull request, approved by the maintainer |
| Repository settings (branch protection, tag protection, required checks, labels) | This specification | Maintainer, running `scripts/setup_repository.sh`, which the specification session provides before implementation starts | Maintainer only |

- `CODEOWNERS` assigns `.github/`, `deny.toml`, lint configuration and `CODEOWNERS` itself to the maintainer.
- An implementation pull request MUST NOT weaken a check (disable a lint, skip a test, widen an allowlist). If a check blocks legitimate work, the implementer opens a specification issue.

## 6.9 Language

- **WF-9.1** English is the only language of the repository: code, identifiers, comments, documentation, commit messages, branch names, issues, pull requests, review comments and error messages.
- **WF-9.2** The rule applies to every session and every account, whatever language the maintainer uses in conversation.
- **WF-9.3** The issue and pull request templates are written in English. A CI check flags pull request titles, bodies and commit subjects containing characters outside ASCII letters, digits and punctuation, or common non-English stop words; it warns and does not block. The `review-pr` skill checks the language.
