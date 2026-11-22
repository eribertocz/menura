# Contract: migration report

**Contract version:** 1.0.0 · **Owner:** `menura` · **Producer:** compiler (`migrate`) · **Consumers:** modelers, editors, CI pipelines

## MIG-1 Purpose

`migrate(model, load_options)` rewrites the deprecated forms of a model document into their
current forms (section 4.3 of the Apache Ossie conformance chapter) and reports each rewrite. It
reads only the model: no catalog schema, state, query or security context.

## MIG-2 Content

| Field | Content |
|---|---|
| `producer` | The compiler build, as in the compiled plan (PLN-1) |
| `changes[]` | `{pointer, rule, from, to}`, one per rewrite (MIG-3) |
| `model` | The migrated model document, as a JSON value |

## MIG-3 Rules

- **MIG-3.1** **Reading.** The model is read as `load` reads it up to E0.b (section 4.5, load options `ossie_format` and `model_name`). A document that E0.a or E0.b rejects returns that compile error, and no report.
- **MIG-3.2** **Rewrites.** The rewrites of this version are:
  - `MIG-3.2a` A `custom_extensions` entry (`nonportable_extensions` in the development line, MDL-2.4) whose `data.namespace` is the former namespace `semantic_compiler` (CON-0.5) gets the configured namespace (load option `namespace`, default `menura`). `pointer` is the RFC 6901 pointer of that `namespace` member in the document as read, `from` is `semantic_compiler`, and `to` is the configured namespace.
  - When the `namespace` load option is `semantic_compiler`, no entry is rewritten (CON-0.5).
  A deprecated extension that a later Apache Ossie release replaces adds a rewrite here, in the release that deprecates it (OSS-3.1).
- **MIG-3.3** **Nothing else changes.** Members, values and list order not named by a rewrite are kept exactly as read, every model of the document included, whatever `model_name` selects. `model` keeps the members of every object in the order they were read.
- **MIG-3.4** **Order.** `changes` is ordered by `pointer` in document order. A document with no deprecated form gives `changes: []` and `model` equal to the document as read.
- **MIG-3.5** **Idempotent.** Migrating the `model` of a report gives `changes: []`.
- **MIG-3.6** **Determinism.** The same document and load options give the same report, byte for byte after canonical JSON (P3).
