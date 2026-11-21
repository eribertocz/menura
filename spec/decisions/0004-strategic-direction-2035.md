# 0004 Strategic direction 2035+: Ossie-first core, agent-native through adapters

## Context

The maintainer approved a strategic direction for Menura (issue #299). Menura is a
semantic compiler that is Apache Ossie-first, deterministic, verifiable, and agent-native
through external adapters. Semantic correctness, security and conformance take absolute
priority over AI integrations.

The approval covers twelve items:
- A, A2: `describe` with authorized `ai_context`, its provenance, and an untrusted label.
- B: structured alternatives.
- C: a delegated actor.
- D: an external MCP adapter.
- E: stable schema URIs.
- F: an agent evaluation bench.
- G: a compatibility matrix.
- H: a Kani pilot.
- I: a provenance manifest.
- J: upstream collaboration.
- K: a WASM study.
- L: property, metamorphic and differential tests.

The governance items M–P and the differentiators D1–D7 are approved only where they show a
real gap.

Each item was reconciled with the rules in force. This record lists only the differences.
Everything else is already specified and is not restated.

## Already specified (no change)

| Item | Rules that already cover it |
|---|---|
| No AI, LLM, MCP, network, clock or I/O in the core | P2, A2, ARC-5.3, vision non-goals |
| Canonical JSON, exact decimals and large integers in hashes and parameters | CON-2 (RFC 8785), CON-4, SEC-3.3 |
| No leak of hidden objects in errors or `describe` | E2-4.4, PLN-5.2, DSC-3.1, DSC-3.2, DSC-3.2a |
| `ai_context` is kept and never affects compilation | E0B-3.5 |
| `ai_context` is exposed only for authorized objects | DSC-2, DSC-3.1 |
| Determinism under key order and separate processes | P3, T4, 7.1a Determinism |
| Typed rejection outside the version's slice | VS-1–VS-4, E2-5.4 |
| Versioned contracts and compatibility | CON-0.3, CON-3, 7.1a Compatibility |
| Migration and per-definition hashes (basis for O and D4) | E0H-3, CON-2 |
| Verified questions (D6) and the natural-language translator | F26, outside the core |
| Agent-driven model editing (D7) | Outside the core. It uses only public contracts, so nothing to add |

## Decision

Each difference gets a rule change or a backlog entry. None of them moves E3/E4 out of 0.4.0.

| Ref. | Difference | Rules affected | Version | Priority |
|---|---|---|---|---|
| Item A2 | `ai_context` entries in `describe` carry their source (the model pointer and `model_hash`) and a versioned trust label `untrusted_model_content`. The label is metadata, not sanitization. Hosts present the text as data, never as instructions. | DSC-2, new DSC-3.6 | 0.7.0 | P1 |
| Item A | No `describe` slice before 0.7.0 unless an issue shows it touches neither E3 nor E4. | 7.1 row 0.7.0 (unchanged) | 0.7.0 | P1 |
| Item B | Diagnostics MAY carry structured, optional alternatives. Every alternative names an authorized object, and their number and length are bounded. An answer with alternatives is indistinguishable for hidden and nonexistent names. Adding the optional member later is a minor contract version, so nothing is needed in 0.4.0. | PLN-5, PLN-5.2 | with `describe` (0.7.0) | P1 |
| Item C | The host may pass a delegated `actor`, authenticated by the host, as invocation metadata of `compile`, never inside the security context. It is recorded in the audit record only. It changes neither the plan, the `plan_hash` nor the security fingerprint. An `actor` inside a query or model is untrusted and ignored. | PLN-6, SEC-3.6 | 0.7.0 (audit), contract before 1.0.0 | P1 |
| Item D1 | A semantic-assurance evidence envelope is an **export of** the audit record (PLN-6) and the plan, built by an adapter. It has no new core format. The core adds nothing except what PLN-6 lacks. The executor adds the data versions and the result digest outside the core. | PLN-6 | after 0.7.0 | P1 |
| Item D, I | An MCP adapter (discover, validate, compile, explain; read-only, no execution) and an optional in-toto Statement v1 export are external modules, each with its own release cycle. They are new backlog entries. | Architecture module table, backlog F29, F30 | after 0.7.0 | P1 / P3 |
| Item E | A schema `$id` is an identifier, never dereferenced. Validation uses the schemas embedded in the release (offline). | CON-0.3 | before 1.0.0 | P1 |
| Item M, D5 | A versioned capability profile: contract versions, Ossie profile, dialects, supported constructs and limits. It is derived from the slice of 7.1c and checked in CI, never inferred. It is exposed through `describe`. | DSC-2, 7.1c | 0.7.0 | P1 |
| Item F | Before 1.0.0, a smoke test with one external agent or client that uses only the public interface: discover, compile, a typed error and a refused metric. The CI uses a deterministic stub. It does not block a release. F26 keeps the full bench. | 7.1 row 0.9.0 | 0.9.0 | P1 |
| Item G | 1.0.0 publishes a compatibility matrix per implementation and channel, backed by suite results. It is never a universal claim. | 7.1 row 1.0.0 | 1.0.0 | P1 |
| Item L, D2 | Differential tests compare result rows with an independent oracle, not SQL text, starting with the 0.4.0 multi-dataset cases. Seeds are recorded and divergences classified. **duckdb-ossie** is a candidate oracle, to be evaluated; it is test tooling only (ARC-5.3), approved in `deny.toml` before use. Metamorphic tests are added per feature. | T4, CS-2.4, new CS-4.x | from 0.4.0, incremental | P0 |
| Item H | A limited Kani pilot on small pure functions, with stated properties and limits. It is never presented as a global proof. Backlog entry. | backlog F31 | exploratory | P3 |
| Item K | A `wasm32` profile study only. It is never required. Backlog entry. | backlog F32 | exploratory | P3 |
| Item J | Upstream collaboration with Apache Ossie: suite comparisons, contributions, documented differences. Menura never calls itself the official reference implementation. | OSS-1 (process) | continuous | P1 |
| Item O, D4 | A semantic diff that classifies model changes (compatible, possibly changing, incompatible, unproven) is an external service over `migrate` and per-definition hashes. Backlog entry. | backlog F33 | after semver base | P2 |
| Item N, P | All channels translate to the public typed requests, and the SQL form is a query form, not an agent protocol. Menura builds no natural-language parser, RDF engine, embedding store, A2A agent, UI or LLM SDK. These are added to the vision non-goals. | 01-vision non-goals, architecture | now (text only) | P1 |

**Priorities (study section 13)**
- **P0:** correct multi-dataset E3/E4 with join and row-policy security, tested on result rows (0.4.0); typed contracts that keep precision, errors and permissions; independent numeric oracles, from 0.4.0.
- **P1:** authorized `describe` and capabilities; `ai_context` with provenance; explanation, lineage and evidence; per-dialect conformance reports; the MCP adapter; the external agent smoke test.
- **P2:** the F26 bench corpus, semantic diff, advanced time logic, and further dialects with equivalence corpora.
- **P3:** ontology (F23), as-of relationships, WASM, A2A, and DAX/MDX/XMLA. These are investigated only.

**Deferred** until a technical justification exists: A2A, mandatory WASM, full formal
verification, in-toto inside the core, and any "100 %" promise.

## Consequences

- 0.4.0 stays centred on E3/E4. Its only additions are the differential-test oracle and
  metamorphic cases for multi-dataset queries.
- No 0.4.0 contract change is needed. Items B and C add optional members later, which is a
  minor version.
- 0.7.0 gains the DSC provenance, trust and capability rules, the audit `actor`, and optional
  alternatives.
- The core stays pure. MCP, evidence export, in-toto and the semantic diff live in adapters.

## Rules changed

To be changed by follow-up specification pull requests:
- DSC-2, DSC-3.6 (new);
- PLN-5, PLN-6;
- CON-0.3;
- CS-4 (new item);
- 7.1 rows 0.9.0 and 1.0.0;
- the vision non-goals;
- the architecture module table;
- backlog entries F29–F33.
