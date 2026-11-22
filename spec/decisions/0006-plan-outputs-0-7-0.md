# 0006 Plan outputs for 0.7.0: lineage, explanation, `explain` and the audit record

## Context

Version 0.7.0 fills the compiled plan's `lineage` and `explanation`, adds `explain`, and
removes the `E_NOT_YET_SUPPORTED` answer to `compile` with `audit: true` (VS-5). The 0.7.0
audit of the plan contract found that these outputs were described but not defined precisely
enough to test: lineage had no entry format or order, the explanation had no stable code or
subject and no list of recorded events, E5 did not say when a rewrite counts as "considered",
`explain` did not say how it relates to `compile`, `data_versions` did not say which periods
are "read", and the audit record left its hashes, its rule names and its unadmitted-context case
open. Decision 0004 also scheduled two items for 0.7.0: structured alternatives in diagnostics
(item B) and a delegated actor in the audit record (item C).

## Decision

- **Lineage (G1).** One entry per `result_schema` column, in its order. `model_inputs` lists
  the transitive fields, metrics and relationships as `field:<dataset>.<field>`,
  `metric:<name>` and `relationship:<name>`, sorted. `physical_inputs` lists every catalog column
  read for the value, its filters, joins and policies, sorted by source and column; a branch
  served by a summary lists the summary relation. `security_policies` lists
  `<dataset>.<policy>`, propagated policies included, sorted. Why a summary was valid moves to
  the explanation.
- **Explanation (G2).** Each entry has a stable `code` (the rule ID deciding it), a `step`
  (`E0` is added for DEV-4), a stable `subject` and a `reason`; the 1.1.0 `decision` member is
  kept and is exactly `applied` or `rejected` for E5 rewrites. Entries are ordered by step,
  then emission order. The recorded events form a closed list: DEV-4; ignored roles and
  attributes (SEC-3.2, SEC-3.2a); resolved relative ranges; accepted `paths` entries (E3-3.3);
  deduplication (E4-4.0e); possible NULL groups (E4-4.1); propagation (E4-6.10); every E5
  rewrite applied, and every one rejected while its structural precondition held; and the
  authorized-executor mark of a summary (E5-4 item 5).
- **Host-facing outputs (G3).** Lineage and the explanation may name summaries, policies and
  datasets joined only for propagation, consistently with E4-6.10 and SCY-4a.1, but never data,
  attribute or query values.
- **`explain` (G4).** It returns exactly the document `compile` returns, with `sql` `""` and
  `parameters` `[]`, and the `cache_key` of the real plan; it returns identical errors and
  ignores `audit`.
- **Periods read (G5).** A period is read when its relation is scanned. Before 1.0.0 a
  `date_range` prunes no period.
- **Audit record (G6).** `inputs.model` is the model hash; the other input hashes are SHA-256
  over RFC 8785 canonical JSON, or over the raw bytes of a document that is not JSON. `subject`
  and `security_fingerprint` are `null` when the context is not admitted. `object` and `rule`
  take a closed vocabulary, entries follow the E2 visit order, and the record gains
  `compile_options` and `load_options.data_source`.
- **Alternatives (0004 item B).** An `E_NAME_NOT_FOUND` diagnostic carries up to 5
  `alternatives`, chosen by edit distance among authorized objects only, so a hidden and a
  nonexistent name get the same answer.
- **Delegated actor (0004 item C).** `actor` is a compile option, written only in the audit
  record and outside every hash.
- The compiled plan, compile error and audit record contracts move to 1.2.0. Every new member is
  optional in the schemas (architecture section 3.3, item 2); the contract text states when the
  compiler writes it.

The relative-range entry is placed at step `E2`, where QRY-4.2 resolves the range (E1 only
records it), and ignored roles and attributes are also `E2`, the first stage that reads them;
both come before every other E2 entry.

## Consequences

Hosts can test lineage and the explanation entry by entry, and audit records can be compared
across runs. An implementation that already emitted empty `lineage` and `explanation` lists
stays valid for 1.1.0 documents. The reason text is not a contract: hosts rely on `code`,
`step`, `subject` and `decision`. Decision 0004 items B and C are implemented; its other items
stay open.

## Rules changed

PLN-1, PLN-3.3a (new), PLN-4, PLN-4.2, PLN-4.3–PLN-4.9 (new), PLN-5, PLN-5.4, PLN-5.4a (new),
PLN-6, PLN-6.3–PLN-6.8 (new), PLN-7, PLN-7.1–PLN-7.4 (new), PLN-8.1–PLN-8.3 (new), E5-2.2,
E5-3.8, architecture sections 3.5 and 3.9.
