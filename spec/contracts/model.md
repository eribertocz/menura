# Contract: semantic model

**Owner:** Apache Ossie (core) and `semantic_compiler` (extensions) · **Producer:** modeling · **Consumer:** compiler

## MDL-1 Format

- **MDL-1.1** A model is one Apache Ossie document, in YAML or JSON, valid against the official Apache Ossie 0.1.1 schema, or against the development-line schema with the load option `ossie_format: development` (section 4.5).
- **MDL-1.2** A document is read the way the official Apache Ossie loader reads it. The exact reading rules (duplicate keys, merges, encodings) are specified in stage E0.

## MDL-2 Extensions

Every capability outside Apache Ossie is carried in the object's `custom_extensions` entry
(Apache Ossie 0.1.1), in this exact shape:

```yaml
custom_extensions:
  - vendor_name: COMMON
    data: >-
      {"namespace": "semantic_compiler", "version": 1, "<extension key>": { ... }}
```

- **MDL-2.1** At most one `semantic_compiler` entry per object. Other vendors' entries are preserved and ignored.
- **MDL-2.2** `data` MUST be a JSON object with `namespace` and `version`. An unknown `version` is an error.
- **MDL-2.3** The namespace value is configurable at build time and defaults to `semantic_compiler`; all rules refer to it by that default.
- **MDL-2.4** In the Apache Ossie development line the same entry is called `nonportable_extensions` and `data` is an object. The reader accepts that form only with `ossie_format: development` (section 4.5), and `semantic-compiler migrate` converts between them.
- **MDL-2.5** A `COMMON` entry whose `namespace` differs from the configured one is someone else's and is kept uninterpreted (E0C-1.1), in models and in queries alike.

| Extension key | Object | Specified in |
|---|---|---|
| `additivity` | Metric | [additivity](../extensions/additivity.md) |
| `populations` | Dataset | [population](../extensions/population.md) |
| `derived` | Metric | [derived](../extensions/derived.md) |
| `context` | Metric | [context](../extensions/context.md) |
| `security_context` | Model | [security](../extensions/security.md) |
| `access` (with optional `fields`), `row_access` | Dataset | [security](../extensions/security.md) |
| `aggregate` | Dataset | [summaries](../extensions/summaries.md) |

## MDL-3 Hash

- **MDL-3.1** The model hash is defined in E0H-3.2.
