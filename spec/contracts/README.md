# Contracts

Every document that crosses a module boundary. Each has a JSON Schema in `schemas/` and valid
examples in `examples/`, all checked in CI.

| Contract | `contract` value | Direction | Chapter |
|---|---|---|---|
| Semantic model | Apache Ossie document | In | [model.md](model.md) |
| Catalog schema | `semantic_compiler.catalog_schema` | In | [catalog.md](catalog.md) |
| Catalog state | `semantic_compiler.catalog_state` | In | [catalog.md](catalog.md) |
| Query | `semantic_compiler.query` | In | [query.md](query.md) |
| Security context | `semantic_compiler.security_context` | In | [security-context.md](security-context.md) |
| Compiled plan | `semantic_compiler.compiled_plan` | Out | [compiled-plan.md](compiled-plan.md) |
| Compile error | `semantic_compiler.compile_error` | Out | [compiled-plan.md](compiled-plan.md) |
| Audit record | `semantic_compiler.audit_record` | Out | [compiled-plan.md](compiled-plan.md) |
| Model description | `semantic_compiler.model_description` | Out | [model-description.md](model-description.md) |
| Integrity checks | `semantic_compiler.integrity_checks` | Out | [integrity-checks.md](integrity-checks.md) |

## Public contracts

- **CON-0.1** Every contract listed above is **public**: it is the only interface other modules may rely on. Anything not in a contract (internal types, plan nodes, crate APIs other than the public functions) is private and may change at any time.
- **CON-0.2** A contract is published as its chapter, its JSON Schema and its examples, in this repository, under the Apache 2.0 license. Any module, in any language, can implement a producer or consumer from them alone, without depending on the compiler's code.
- **CON-0.3** Each release attaches every JSON Schema as a release asset (the `release-assets` workflow, CI-2), and each schema keeps a stable `$id` per contract version. The `$id` base (`https://semantic-compiler.invalid/`) is a placeholder, replaced by the project's public URL before 1.0.0.
- **CON-0.4** Changing a public contract follows the evolution rules of architecture section 3.3.

## Shared rules

- **CON-1** JSON Schema 2020-12. All objects closed (`additionalProperties: false`), except Apache Ossie objects, which follow the Apache Ossie schema.
- **CON-2** Hashes are SHA-256 over RFC 8785 canonical JSON, written as 64 lowercase hex digits, with no prefix, in every contract.
- **CON-3** Versions evolve as in architecture section 3.3.
- **CON-4** Value encoding. Because RFC 8785 serializes numbers as IEEE-754 doubles, every value whose exact value matters is a JSON **string** in canonical form: integers outside ±(2^53 − 1), every decimal (`"-12.50"`: optional minus, digits, `.` and exactly `scale` digits; with scale 0 there is no `.`; the integer part has no leading zeros except a single `0`; zero is never negative, so it is written `0`, `0.00`, …), dates (`YYYY-MM-DD`), times (`HH:MM:SS` plus `.` and 1–9 fraction digits only when non-zero, trailing zeros removed), timestamps (`YYYY-MM-DDTHH:MM:SS` with the same fraction rule; with `with_time_zone`, converted to UTC and suffixed `Z`; without, no suffix), and UUIDs (lowercase, hyphenated). Integers within ±(2^53 − 1), booleans and NULL use JSON numbers, booleans and `null`. Floats use JSON numbers. This applies to parameters, security attributes and every hashed document.
- **CON-5** Ordering. "Ordered by name" means ordered by the normalized identifier compared as a sequence of Unicode code points. Identifier normalization (E0D-1) uses Unicode simple upper-case mapping, so the length of an identifier never changes.
