# Contracts

Every document that crosses a module boundary. Each has a JSON Schema in `schemas/` and valid
examples in `examples/`, all checked in CI.

| Contract | `contract` value | Direction | Chapter |
|---|---|---|---|
| Semantic model | Apache Ossie document | In | [model.md](model.md) |
| Catalog schema | `menura.catalog_schema` | In | [catalog.md](catalog.md) |
| Catalog state | `menura.catalog_state` | In | [catalog.md](catalog.md) |
| Query | `menura.query` | In | [query.md](query.md) |
| Security context | `menura.security_context` | In | [security-context.md](security-context.md) |
| Compiled plan | `menura.compiled_plan` | Out | [compiled-plan.md](compiled-plan.md) |
| Compile error | `menura.compile_error` | Out | [compiled-plan.md](compiled-plan.md) |
| Audit record | `menura.audit_record` | Out | [compiled-plan.md](compiled-plan.md) |
| Model description | `menura.model_description` | Out | [model-description.md](model-description.md) |
| Integrity checks | `menura.integrity_checks` | Out | [integrity-checks.md](integrity-checks.md) |
| Model dependencies | `menura.model_dependencies` | Out | [model-dependencies.md](model-dependencies.md) |
| Model diff | `menura.model_diff` | Out | [model-dependencies.md](model-dependencies.md) |
| Migration report | `menura.migration_report` | Out | [migration-report.md](migration-report.md) |

## Public contracts

- **CON-0.1** Every contract listed above is **public**: it is the only interface other modules may rely on. Anything not in a contract (internal types, plan nodes, crate APIs other than the public functions) is private and may change at any time.
- **CON-0.2** A contract is published as its chapter, its JSON Schema and its examples, in this repository, under the Apache 2.0 license. Any module, in any language, can implement a producer or consumer from them alone, without depending on the compiler's code.
- **CON-0.3** Each release attaches every JSON Schema as a release asset (the `release-assets` workflow, CI-2), and each schema keeps a stable `$id` per contract version. The `$id` base (`https://menura.invalid/`) is a placeholder, replaced by the project's public URL before 1.0.0. A `$id` is an identifier, never dereferenced: validation uses the schemas embedded in the release and works offline (P2).
- **CON-0.4** Changing a public contract follows the evolution rules of architecture section 3.3.
- **CON-0.5** Former name. The project is **Menura**; its former name was `semantic_compiler`. Through major version 1 of the contracts, an input document whose `contract` is `semantic_compiler.<name>` is read exactly as `menura.<name>`, with a deprecation warning: `WCAT-05` for catalog documents, `WQRY-01` for a query and for a security context. The extension namespace `semantic_compiler` is accepted beside the configured one, with `W0C-02` (E0C-1). The compiler always writes output documents with `menura.<name>`. Through major version 1 every schema also accepts the former name, so documents written before the rename (an audit record, for example) still validate. The command-line binary `semantic-compiler` remains an alias of `menura`, with the same arguments and results. Removing an alias narrows the schemas, so every alias is removed together at the next major version (2.0.0), whose release notes announce the removal (architecture section 3.3, item 6); until then the warnings above are the deprecation notice.
  - `WCAT-05` is located at `/contract` of its document. It is reported only for a document that is otherwise admitted (a rejected schema or an ignored state already has its own code). A schema's `WCAT-05` comes before the E0 warnings (E0-3.2), and a state's comes after them (ARC-5.2).
  - `W0C-02` is located like every E0.c diagnostic, at the entry's `data` member (E0C-2.6). It is reported for every entry read through the former namespace, whatever else that entry reports. When the `namespace` load option is set to `semantic_compiler`, that namespace is the configured one, and no `W0C-02` is reported.
  - Queries and security contexts already accept both names through their schemas. `WQRY-01` arrives with E1 (0.3.0).

## Shared rules

- **CON-1** JSON Schema 2020-12. All objects closed (`additionalProperties: false`), except Apache Ossie objects, which follow the Apache Ossie schema.
- **CON-2** Hashes are SHA-256 over RFC 8785 canonical JSON, written as 64 lowercase hex digits, with no prefix, in every contract.
- **CON-3** Versions evolve as in architecture section 3.3.
- **CON-4** Value encoding. Because RFC 8785 serializes numbers as IEEE-754 doubles, every value whose exact value matters is a JSON **string** in canonical form: integers outside ±(2^53 − 1), every decimal (`"-12.50"`: optional minus, digits, `.` and exactly `scale` digits; with scale 0 there is no `.`; the integer part has no leading zeros except a single `0`; zero is never negative, so it is written `0`, `0.00`, …), dates (`YYYY-MM-DD`), times (`HH:MM:SS` plus `.` and 1–9 fraction digits only when non-zero, trailing zeros removed), timestamps (`YYYY-MM-DDTHH:MM:SS` with the same fraction rule; with `with_time_zone`, converted to UTC and suffixed `Z`; without, no suffix), and UUIDs (lowercase, hyphenated). Integers within ±(2^53 − 1), booleans and NULL use JSON numbers, booleans and `null`. Floats use JSON numbers. This applies to parameters, security attributes and every hashed document.
- **CON-5** Ordering. "Ordered by name" means ordered by the normalized identifier compared as a sequence of Unicode code points. Identifier normalization (E0D-1) uses Unicode simple upper-case mapping, so the length of an identifier never changes.
