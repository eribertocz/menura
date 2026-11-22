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
| `model` | The migrated model document (MIG-3.3) |

## MIG-3 Rules

- **MIG-3.1** **Reading.** The model is read by E0.a and E0.b (section 4.5, load option `ossie_format`), except the model selection of E0B-3.2: `migrate` covers every model of the document, so `model_name` is ignored and a document with several models needs none. A document that E0.a or E0.b rejects returns that compile error, and no report. Warnings of E0.a and E0.b are not reported; `validate` reports them.
- **MIG-3.2** **Rewrites.** The rewrites of this version are:
  - `MIG-3.2a` **Former namespace.** An extension entry is rewritten when its `vendor_name` is `COMMON`, its `data` parses as a JSON object (E0C-1, conditions 1 and 2), and that object has exactly one `namespace` member, the string `semantic_compiler` (CON-0.5). Its `namespace` becomes the configured namespace (load option `namespace`, default `menura`); `from` is `semantic_compiler` and `to` the configured namespace. In the release format (`custom_extensions`, `data` a JSON string, MDL-2) only the characters of the `namespace` value's JSON string token inside `data` are replaced, every other character of `data` is kept, and `pointer` is the RFC 6901 pointer of the `data` member. In the development format (`nonportable_extensions`, `data` an object, MDL-2.4) `pointer` is that of the `namespace` member. Every other entry is left unchanged, with no change recorded.
  - When the `namespace` load option is `semantic_compiler`, no entry is rewritten (CON-0.5).
  A deprecated extension that a later Apache Ossie release replaces adds a rewrite here, in the release that deprecates it (OSS-3.1).
- **MIG-3.3** **Nothing else changes.** `model` is the document tree that E0.a produces (anchors and aliases expanded, E0A-2.4; dates as text, E0A-2.8), as a JSON value, with only the rewrites of MIG-3.2 applied. Members keep the order they were read in, every model is kept, and the format is not converted. Pointers refer to that tree, so an alias expanded in several places gives one change per place.
- **MIG-3.3a** **Numbers.** `model` writes every integer with its exact digits, whatever its size, and every other number as read. A document holding a number that has no JSON form (`.inf`, `-.inf` or `.nan` in YAML) is refused with the compile error `EMIG-01` ("a number of the model has no JSON form"), located at that value, and gives no report.
- **MIG-3.4** **Order.** `changes` is ordered by `pointer` in document order. A document with no deprecated form gives `changes: []` and `model` equal to the tree as read.
- **MIG-3.5** **Idempotent.** Migrating the `model` of a report gives `changes: []`.
- **MIG-3.6** **Determinism.** The same document and load options give the same report, byte for byte after canonical JSON (P3).
