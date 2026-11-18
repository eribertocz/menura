# E0 Load and validate the model

## E0-1 Purpose

Turn the model document into an immutable validated model, or return every problem found. No
query is ever compiled against a model with errors.

## E0-2 Input and output

| | Content |
|---|---|
| Input | One Apache Ossie document ([model contract](../../contracts/model.md)) and one catalog schema document ([catalog contract](../../contracts/catalog.md)) |
| Output on success | Immutable model ([internal-model.md](internal-model.md)), warnings, recorded integrity checks, summary definition hashes |
| Output on failure | Compile error document with all errors and warnings |

- **E0-2.1** E0 needs no security context and no catalog state.

## E0-3 Substages

Run in this order. A substage runs only if the substages it depends on succeeded.

| Substage | Depends on | File |
|---|---|---|
| E0.a Read | — | [read.md](read.md) |
| E0.b Validate structure | E0.a | [structure.md](structure.md) |
| E0.c Validate extensions | E0.b | [extensions.md](extensions.md) |
| E0.d Resolve references | E0.b, catalog schema valid | [references.md](references.md) |
| E0.e Analyze relationships | E0.d | [relationships.md](relationships.md) |
| E0.f Type expressions | E0.d | [expressions.md](expressions.md) |
| E0.g Classify metrics | E0.c, E0.f | [metrics.md](metrics.md) |
| E0.h Build internal model | All above | [internal-model.md](internal-model.md) |

## E0-4 Errors and unusable elements

- **E0-4.1** Each E0 diagnostic has one **disposition**: *reject* (the model does not load), or *unusable* (the model loads, the element is marked unusable, and a load warning is reported). A query that names an unusable element fails with the element's **query error code**.
- **E0-4.2** A problem with a construct of the compiler's own (extensions, catalog) or a declared deviation (section 4.6) rejects the model. A problem with a construct Apache Ossie accepts makes the element unusable. An unusable relationship is excluded from the graph; an unusable dataset makes its fields, metrics and relationships unusable.
- **E0-4.3** Within E0, diagnostics are reported in substage order, then in document order (JSON Pointer order), then by code.

| Cause | Disposition | Load warning | Query error |
|---|---|---|---|
| `E0A-*` (reading), `E0B-01`–`E0B-07` (structure, as the official schema requires) | Reject | — | — |
| Expression without a single `ANSI_SQL` variant (E0B-3.3) | Unusable | `W0B-02` | `E0B-08` |
| Unknown development-line `datatype` (OSS-5.2) | Unusable | `W0B-03` | `E0B-09` |
| `E0C-*` (extensions), except `E0C-37` | Reject | — | — |
| `E0C-37` (`AGG()` of a model metric outside derived and context metrics) | Unusable | `W0G-01` | `E0C-37` |
| `E0D-01` duplicate names (DEV-2), `E0D-02` source not in catalog (DEV-3), `E0D-08` | Reject | — | — |
| `E0D-03`–`E0D-07` (missing references) | Unusable | `W0D-01` | The `E0D` code |
| `E0E-01`, `E0E-02` (relationships) | Unusable (relationship) | `W0E-04` | `E_NAME_NOT_FOUND` when named, otherwise the path errors of E3 |
| `E0F-*` (expressions and types, including outside the subset) | Unusable | `W0F-01` | The `E0F` code |
| Window function in a metric (E0F-2.4, E0G-2.2) | Unusable | `W0F-01` | `E_DEFERRED_WINDOWED_METRIC` |
| Apache Ossie metric errors (E0G-2.1, E0G-2.3–E0G-2.5) | Unusable | `W0G-01` | The Apache Ossie code |
| `E0G-01`, `E0G-02` | Unusable | `W0G-01` | The `E0G` code |

- **E0-4.4** The 0.1.1 reference validator also rejects duplicate names and missing references; the compiler's *unusable* disposition for missing references is therefore more lenient than Apache Ossie and never rejects a document Apache Ossie accepts (OSS-2.1).

| Extension code range | Substage | Depends on |
|---|---|---|
| Shape and keys of every extension (`E0C-01`–`E0C-05`, `E0C-10`, `E0C-11`, `E0C-13`–`E0C-15`, `E0C-20`, `E0C-21`, `E0C-36`, `E0C-41`–`E0C-45`, `E0C-47`, `E0C-55`, `E0C-56`, `E0C-62`) | E0.c | E0.b |
| References inside extensions (`E0C-22`, `E0C-32`, `E0C-33`, `E0C-37`, `E0C-49`–`E0C-51`, `E0C-53`, `E0C-54`, `E0C-60`, `E0C-63`, `E0C-64`) | E0.d | E0.c |
| Reachability inside extensions (`E0C-16`, `E0C-61`) | E0.e | E0.d |
| Expressions and types inside extensions (`E0C-12`, `E0C-17`, `E0C-23`, `E0C-24`, `E0C-30`, `E0C-31`, `E0C-34`, `E0C-35`, `E0C-46`, `E0C-48`, `E0C-52`) | E0.f | E0.e |

Extension-specific validation (additivity, population, derived, context, security, summaries)
is specified in [`spec/extensions/`](../../extensions/README.md) and runs within E0.c, E0.d and E0.f.
