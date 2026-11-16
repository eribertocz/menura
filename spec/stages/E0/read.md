# E0.a Read

## E0A-1 Purpose

Turn the document text into a data tree that has exactly one reading.

## E0A-2 Rules

The document is read as the official Apache Ossie reference loader reads it (YAML 1.1 safe loading), with one declared deviation: duplicate keys are an error (DEV-1).

| ID | Rule |
|---|---|
| E0A-2.1 | Encoding is UTF-8. |
| E0A-2.2 | YAML 1.1 safe loading: only standard tags; no arbitrary object construction. JSON documents are read as YAML. |
| E0A-2.3 | Duplicate keys are an error, at every nesting level. |
| E0A-2.4 | Anchors, aliases and the merge key `<<` are allowed and expanded. |
| E0A-2.5 | A tag or value the loader cannot construct (`!!int abc`, an invalid date) is an error. |
| E0A-2.6 | Exactly one YAML document per file. |
| E0A-2.7 | Every node keeps its line and column for diagnostics. |
| E0A-2.8 | Scalars that YAML 1.1 reads as non-JSON values (dates, timestamps) are converted to their ISO 8601 text form before any hashing or validation. |

Because YAML 1.1 reads `yes`, `no`, `on` and `off` as booleans, text values that could be read
that way should be quoted. The reader does not reinterpret them.

## E0A-3 Errors

| Code | Condition |
|---|---|
| `E0A-01` | Not valid UTF-8 |
| `E0A-02` | YAML cannot be parsed |
| `E0A-03` | More than one YAML document |
| `E0A-04` | Duplicate key |
| `E0A-05` | Tag or value cannot be constructed |
