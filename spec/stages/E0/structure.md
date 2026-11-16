# E0.b Validate structure

## E0B-1 Reference

- **E0B-1.1** The structure is the official Apache Ossie 0.1.1 JSON Schema (`core-spec/osi-schema.json`). Where the prose specification and the JSON Schema differ, the JSON Schema wins.
- **E0B-1.2** Objects are closed exactly as the official schema declares them (`AIContext` is open).

## E0B-2 Apache Ossie 0.1.1 structure

`*` = required.

| Object | Properties |
|---|---|
| Root | `version`*, `semantic_model`*, `dialects`, `vendors` |
| SemanticModel | `name`*, `datasets`* (min. 1), `relationships`, `metrics`, `description`, `ai_context`, `custom_extensions` |
| Dataset | `name`*, `source`*, `primary_key`, `unique_keys`, `fields`, `description`, `ai_context`, `custom_extensions` |
| Relationship | `name`*, `from`*, `to`*, `from_columns`* (min. 1), `to_columns`* (min. 1), `ai_context`, `custom_extensions` |
| Field | `name`*, `expression`*, `dimension` (`is_time`), `label`, `description`, `ai_context`, `custom_extensions` |
| Metric | `name`*, `expression`*, `description`, `ai_context`, `custom_extensions` |
| Expression | `dialects`* (min. 1) of `{dialect*, expression*}` |
| CustomExtension | `vendor_name`*, `data`* (a string holding JSON) |

- `dialect` and `vendor_name` take the values enumerated in the official schema. The compiler compiles only `ANSI_SQL` expressions and interprets only `COMMON` extensions.
- In 0.1.1, metrics are declared at model level only.

## E0B-3 Compiler rules on structure

| ID | Rule |
|---|---|
| E0B-3.1 | `version` MUST be `"0.1.1"`. |
| E0B-3.2 | If `semantic_model` has more than one element, the load option `model_name` MUST name one (`E0B-07`); the others are ignored (DEV-7). |
| E0B-3.3 | The `ANSI_SQL` variant of an Expression is the only one compiled. An Expression without one, or with more than one, makes its field or metric unusable (`W0B-02`, OSS-2.2). |
| E0B-3.4 | Variants in other dialects are kept, not compiled, and produce `W0B-01`. |
| E0B-3.5 | `dialects`, `vendors` and `ai_context` are kept and never affect compilation. |
| E0B-3.6 | Diagnostics locate the node by JSON Pointer, e.g. `/semantic_model/0/datasets/2/fields/1/expression`. |

Name uniqueness and existence of references are checked in E0.d, not here.

## E0B-4 Diagnostics

| Code | Condition |
|---|---|
| `E0B-01` | `version` missing or not `"0.1.1"` |
| `E0B-02` | Required property missing |
| `E0B-03` | Wrong type |
| `E0B-04` | Property not allowed |
| `E0B-05` | Value outside an enumeration |
| `E0B-06` | Empty list where at least one item is required |
| `E0B-07` | Several semantic models and no `model_name`, or `model_name` names none of them |
| `W0B-01` | Variant in a dialect other than `ANSI_SQL`; not compiled |
| `W0B-02` | No single `ANSI_SQL` variant; the field or metric is unusable |
| `W0B-03` | Unknown development-line `datatype`; the element is unusable |
| `E0B-08` | Query names an element without a single `ANSI_SQL` variant |
| `E0B-09` | Query names an element with an unknown `datatype` |
