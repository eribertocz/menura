# E0.d Resolve references

## E0D-1 Identifiers

| ID | Rule |
|---|---|
| E0D-1.1 | An unquoted identifier is case-insensitive and normalized to upper case. A double-quoted identifier is compared exactly. |
| E0D-1.2 | An identifier has at most 128 characters. |
| E0D-1.3 | Two names in the same scope that are equal after normalization are the same name. |

These rules are those of the Apache Ossie development line; Apache Ossie 0.1.1 does not define
identifier comparison, and the compiler adopts them now so that models stay valid when they
become normative.

## E0D-2 Uniqueness

| Name | Unique within |
|---|---|
| `datasets[].name` | The model |
| `fields[].name` | Its dataset |
| `metrics[].name` | The model |
| `relationships[].name` | The model |
| Names declared by extensions (populations, policies) | The model |

A metric and a dataset may share a name: metric references and field references never share a syntactic position.

## E0D-3 Existence

| Reference | MUST exist in |
|---|---|
| `datasets[].source` | The catalog schema, with identical text |
| Columns read by a field expression | The catalog relation of the dataset's `source`. A field expression reads source columns only, never other fields. |
| `dataset.field` in a metric expression | A dataset of the model and one of its fields |
| `relationships[].from`, `to` | Datasets of the model |
| `primary_key`, `unique_keys`, `from_columns`, `to_columns` | Fields of the corresponding dataset |
| References inside extensions | As defined by each extension |

- **E0D-3.1** A name in `primary_key`, `unique_keys`, `from_columns` or `to_columns` resolves to the dataset field of that name (development-line rule); if no field has it, to the `source` column of that name (0.1.1 wording). If both exist and the field is not exactly that column, it is an error (`E0D-07`). The type is checked in E0.f.
- **E0D-3.2** When an extension defines a specific code for a missing reference, that code is used instead of the generic one.

## E0D-4 Diagnostics

| Code | Condition |
|---|---|
| `E0D-01` | Duplicate name, or name longer than 128 characters |
| `E0D-02` | Dataset `source` not in the catalog schema |
| `E0D-03` | Field expression reads a column not in its catalog relation |
| `E0D-04` | Metric refers to a missing dataset or field |
| `E0D-05` | Relationship refers to a missing dataset |
| `E0D-06` | Key or relationship column names neither a field nor a source column |
| `E0D-07` | Key name is both a field and a different source column |
| `E0D-08` | A `source` that is a query is not a single read-only `SELECT` |
| `W0D-01` | A missing reference; the element is unusable (E0-4.1) |
