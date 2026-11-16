---
name: ossie-update
description: Handle an Apache Ossie release or specification change opened by the watcher. Use when an issue labeled ossie-watch is assigned.
---

# Apache Ossie update

1. Read the issue: release or development change, versions or commits, and the attached diff.
2. Implementation session: update `conformance/ossie/` to the new commit, run `just conformance ossie`, and update `conformance/ossie/expected-failures.yaml` with every failing case and the issue that will fix it (OSS-2.4). This is one `impl` pull request.
3. For an **official release**:
   - Specification session: update `spec/ossie-mapping.md` first (OSS-4.5), then compare the release with `spec/04-ossie-conformance.md` and every extension in `spec/extensions/`, in one `spec` pull request.
   - For every extension the release now covers, propose its deprecation and the migration mapping in that `spec` pull request.
   - Open `impl` issues for every failing case, each with the rule it breaks.
4. For a **development change**: update the tracked commit, and open one `impl` issue listing the failing cases, labeled `study`.
5. Comment on the watcher issue with the summary: passing, failing, extensions affected, issues opened.
6. Remember the support window: an official release must be supported within 14 days (section 4.4).
