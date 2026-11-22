# Contract: model description

**Contract version:** 1.2.0 · **Owner:** `menura` · **Producer:** compiler (`describe`) · **Consumers:** hosts, translators, editors

## DSC-1 Purpose

`describe(handle, security_context)` tells a consumer what **this user** may ask: which
datasets, fields and metrics exist for them, how they can be grouped, and which paths need a
choice. Interfaces build their menus from it; translators propose only what it lists.

## DSC-2 Content

| Field | Content |
|---|---|
| `model_hash`, `security_fingerprint` | Identify the model and the context the description is valid for |
| `datasets[]` | Authorized datasets: name, label, description, `ai_context` (DSC-3.6), and their authorized fields: reference, label, description, logical type, `is_time`, the time grains it accepts (DSC-3.8), `ai_context` |
| `metrics[]` | Authorized metrics: reference, label, description, result type, `kind` (`base`, `derived`, `context`), the datasets reachable from it (E3-3.4), `ai_context` |
| `ambiguous_paths[]` | Pairs of datasets with more than one path, with exactly two witness paths each (DSC-3.9), so a consumer can ask the user to choose (the full set is never enumerated) |
| `capabilities` | The capability profile of the compiler release (DSC-4) |

## DSC-3 Rules

- **DSC-3.1** Only objects the security context is authorized to use are listed (E2-4). An unauthorized object is absent, never marked as restricted.
- **DSC-3.2** Summaries (`access: internal`) are never listed.
- **DSC-3.2a** `reachable_datasets` and `ambiguous_paths` are computed over the authorized subgraph (E2-4.8).
- **DSC-3.3** No data values, attribute values or row-policy conditions appear.
- **DSC-3.4** Lists are ordered by normalized name (P3); the same inputs give the same document.
- **DSC-3.5** `describe` is a pure function of the model and the security context; it needs no catalog state.
- **DSC-3.6** **`ai_context` provenance.** An `ai_context` member is present only on a listed dataset, field or metric whose model element declares `ai_context` (DSC-3.1). It is the object `{value, source: {pointer, model_hash}, trust}`:
  - `value` is the element's `ai_context` exactly as the model declares it, read into the JSON data model (E0H keeps it as written); a declared `null` gives `value: null`.
  - `source.pointer` is the RFC 6901 JSON Pointer of that `ai_context` member in the model document as loaded, with list positions as written, for example `/semantic_model/0/datasets/0/fields/5/ai_context`. Like every source location it is outside the model hash (E0H-3.2).
  - `source.model_hash` equals the description's `model_hash`.
  - `trust` is the versioned trust label; contract version 1.2.0 defines one value, `untrusted_model_content`. A new label value is a minor contract version (CON-3).
  - The label is metadata, not sanitization. It never changes authorization, row policies, compilation or the list of objects. A host presents `value` as data, never as instructions.
- **DSC-3.7** **Omitted elements.** An element marked unusable in E0 (E0-4.1, including by propagation, E0-4.2a) is omitted: an unusable dataset with its fields, an unusable field, an unusable metric. A field whose logical type has kind `unsupported` is omitted too. An omitted dataset never appears in `reachable_datasets`, `ambiguous_paths` or a witness path; unusable relationships are already outside the graph (E0H-2.3). A listed dataset whose every field is omitted is listed with `fields: []`.
- **DSC-3.8** **Time grains.** `time_grains` is `["day", "week", "month", "quarter", "year"]`, in that order, for a field whose logical type has kind `date` or `timestamp`, whatever its `is_time`; it is `[]` for every other field. `is_time` is the field's declared `dimension.is_time`, `false` when absent.
- **DSC-3.9** **Witness paths.** `ambiguous_paths` lists every ordered pair (`from`, `to`) of listed datasets with at least two paths from `from` to `to` over the authorized subgraph (E0E-2.3, E0E-2.4, DSC-3.2a), ordered by `from`, then `to`, in CON-5 order. `paths` holds the two smallest paths of the pair, ordered by length (number of relationships), then by the CON-5 order of their sequences of relationship normalized names compared element by element. The member is always present, possibly empty.
- **DSC-3.10** **Path encoding.** A path is the list of the normalized names (E0D-1) of its relationships, in order from `from` to `to`; `from` and `to` are the normalized names of the datasets. A consumer that writes a path into a query `paths` entry (QRY-4.3) writes each name as a delimited identifier, so its case is kept.
- **DSC-3.11** **Optional members.** `label`, `description` and `ai_context` are present only when the model element declares them, and never as `null` (a declared `null` `ai_context` gives the object of DSC-3.6). `label` is the element's declared label.
- **DSC-3.12** **Security context.** `describe` admits the security context exactly as `compile` does (SEC-3.8). A context that is not admitted returns the compile error that SEC-3.8 gives for it (`ESEC-01` or `ESEC-03`, same location), and no description. An admitted context read through the former contract name gives the same description as under `menura.security_context`; `WQRY-01` (SEC-3.9) is not reported, because a description carries no warnings.

## DSC-4 Capability profile

`capabilities` states what this compiler release can compile, so a host never infers a
capability from a model or from a failed query.

| Field | Content |
|---|---|
| `compiler_version` | The release, as `producer.compiler_version` of a compiled plan (PLN-1) |
| `slice_version` | The row of the roadmap table (7.1) whose slice (7.1c) the release compiles, as `MAJOR.MINOR.PATCH`; from 1.0.0 it is `1.0.0` |
| `ossie` | `normative` (the normative Apache Ossie release, `0.1.1`), `tracked_commit` (as `producer.ossie_tracked_commit`) and `formats` (the accepted `ossie_format` load options, section 4.5: `development`, `release`) |
| `contracts[]` | One entry `{contract, version}` per public contract the release reads or writes (`menura.<name>`, CON-0.1), with the latest version it implements; every earlier minor version of the same major is accepted too (CON-3) |
| `dialects[]` | The dialects `compile` and `integrity_checks` accept (E6-3.1), today `duckdb` |
| `query_forms[]` | The query forms the slice compiles: `wide_table` (member `query`, QRY-3) and `sql` (member `sql`, QRY-5) |
| `extensions[]` | One entry `{key, versions}` per extension key of the extensions table whose elements the slice compiles, with the `data.version` values it accepts (MDL-2.2) |
| `functions[]` | The public functions the release provides (architecture section 3.5) |
| `limits` | `max_identifier_length` (E0D-1.2, 128) and `max_decimal_precision` (38, the largest decimal precision of a logical type) |

- **DSC-4.1** `capabilities` is a constant of the release: it depends on neither the model nor the security context, and it is always present.
- **DSC-4.2** The profile is derived from the roadmap, never inferred: `slice_version`, `query_forms`, `extensions` and `functions` list exactly what the slice of `slice_version` compiles and provides (7.1, 7.1c, VS-5). A capability outside the slice is never listed, and a listed one never returns `E_NOT_YET_SUPPORTED`. For example, in 0.7.0 `query_forms` is `["sql", "wide_table"]` and `extensions` has no `context` entry (0.8.0).
- **DSC-4.3** Every list is sorted without duplicates: strings in CON-5 order, `contracts` by `contract`, `extensions` by `key`, `versions` in ascending order. `ossie.formats` is sorted the same way.
- **DSC-4.4** The conformance suite holds the expected profile of each release. `describe` MUST return it byte for byte (RFC 8785 canonical JSON, CON-2), and every listed query form, extension and function MUST have its rules in the coverage scope of the release (CS-2.6). A profile that differs from the expected one, or a listed capability whose rules are not covered, fails CI.
- **DSC-4.5** The profile is metadata. It never changes compilation, authorization or any hash, and it is not part of `security_fingerprint` or `model_hash`.

Schema: [`schemas/model-description.schema.json`](schemas/model-description.schema.json) · Example: [`examples/model-description.example.json`](examples/model-description.example.json)
