# E0.d Resolve references

## E0D-1 Identifiers

| ID | Rule |
|---|---|
| E0D-1.1 | An unquoted identifier is case-insensitive and normalized to upper case. A double-quoted identifier is compared exactly. |
| E0D-1.2 | An identifier has at most 128 characters. |
| E0D-1.3 | Two names in the same scope that are equal after normalization are the same name. |
| E0D-1.4 | A declared name (a YAML string naming a dataset, field, metric, relationship or semantic model, or a key or relationship column) is a delimited identifier when its text starts and ends with `"`, with `""` standing for `"`; it is then compared exactly, without the quotes. Any other declared name is a regular identifier. A delimited name has at least one character between its quotes, and inside them `"` appears only doubled; a text that starts with `"` and breaks either rule (`"a"b"`, `"`, `""`) is malformed: `E0D-01`, because reading it would need a second interpretation (P1). |
| E0D-1.5 | A catalog column name is the column's exact spelling. A regular reference matches the column whose name, normalized as E0D-1.1, equals the reference's normalized name; a delimited reference matches the column whose name is exactly its text (CAT-8.2 makes the first unambiguous). |

These rules are those of the Apache Ossie development line; Apache Ossie 0.1.1 does not define
identifier comparison, and the compiler adopts them now so that models stay valid when they
become normative.

## E0D-2 Uniqueness

| Name | Unique within |
|---|---|
| `semantic_model[].name` | The document (checked even for models not selected) |
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
| A bare name, and `source.column`, in a development-line dataset metric expression (section 4.5) | A field of that dataset (`E0D-04` if missing); a column of the catalog relation of its `source` (`E0D-03` if missing). The qualifier `source` is compared as a regular identifier (E0D-1.1, E0D-1.3); any other qualified reference is `E0F-02` (E0D-3.4) |
| `relationships[].from`, `to` | Datasets of the model |
| `primary_key`, `unique_keys`, `from_columns`, `to_columns` | Fields of the corresponding dataset |
| References inside extensions | As defined by each extension |

- **E0D-3.1** A name in `primary_key`, `unique_keys`, `from_columns` or `to_columns` resolves to the dataset field of that name (development-line rule); if no field has it, to the `source` column of that name (0.1.1 wording). If both exist and the field is not exactly that column, it is an error (`E0D-07`). The type is checked in E0.f.
- **E0D-3.2** When an extension defines a specific code for a missing reference, that code is used instead of the generic one.
- **E0D-3.3** A `source` that is a query passes `E0D-08` only if, outside string literals, quoted identifiers and comments, it has no `;` except one at the end, its first keyword is `SELECT` or `WITH`, and it contains none of these keywords as a word: `INSERT`, `UPDATE`, `DELETE`, `MERGE`, `UPSERT`, `REPLACE`, `CREATE`, `DROP`, `ALTER`, `TRUNCATE`, `COPY`, `ATTACH`, `DETACH`, `INSTALL`, `LOAD`, `PRAGMA`, `SET`, `RESET`, `CALL`, `EXPORT`, `IMPORT`, `INTO`, `VACUUM`, `CHECKPOINT`, `GRANT`, `REVOKE`, `BEGIN`, `COMMIT`, `ROLLBACK`. The check guards the model text; the executor's read-only identity remains the protection at run time.
- **E0D-3.4** A reference form the table above does not list (a qualified reference in a field expression, an unqualified one in a metric expression) is outside the accepted subset: `E0F-02` (E0F-2.2, row "References").

## E0D-4 Diagnostics

| Code | Condition |
|---|---|
| `E0D-01` | Duplicate name, name longer than 128 characters, or malformed delimited name (E0D-1.4) |
| `E0D-02` | Dataset `source` not in the catalog schema |
| `E0D-03` | Field expression reads a column not in its catalog relation |
| `E0D-04` | Metric refers to a missing dataset or field |
| `E0D-05` | Relationship refers to a missing dataset |
| `E0D-06` | Key or relationship column names neither a field nor a source column |
| `E0D-07` | Key name is both a field and a different source column |
| `E0D-08` | A `source` that is a query is not a single read-only `SELECT` |
| `W0D-01` | A missing reference; the element is unusable (E0-4.1) |
