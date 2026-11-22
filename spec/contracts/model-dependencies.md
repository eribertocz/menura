# Contract: model dependencies and model diff

**Contract version:** 1.0.0 · **Owner:** `menura` · **Producer:** compiler (`dependencies`, `diff`) · **Consumers:** impact analysis (Mergus), modelers, editors

## DEP-1 Purpose

`dependencies(handle)` publishes the model-level dependency graph: for each dataset,
relationship, field, metric and row policy, the model elements, catalog columns and row
policies it depends on, and a hash of its own definition. It is computed from the model alone,
never per query, so a consumer can find the impact radius of a change without parsing plans.
`diff(old, new)` compares two such documents and classifies each element as added, removed or
changed.

## DEP-2 Content

**Model dependencies** (`menura.model_dependencies`):

| Field | Content |
|---|---|
| `producer` | The compiler build, as in the compiled plan (PLN-1) |
| `model_hash` | The model hash (E0H-3.2) |
| `datasets[]`, `relationships[]`, `fields[]`, `metrics[]`, `policies[]` | One entry per model element of that kind (DEP-3.2) |

Each entry has:

| Member | Content |
|---|---|
| `object` | `dataset:<name>`, `relationship:<name>`, `field:<dataset>.<field>`, `metric:<name>` (a dataset-scoped metric of the development format, `metric:<dataset>.<metric>`, as a query names it, QRY-3.2) or `policy:<dataset>.<policy>`, with normalized names (E0H-1.1, PLN-7.2) |
| `kind` | Metrics only: `base`, `derived` or `context`, as in the model description (DSC-2) |
| `definition_hash` | The element's definition hash (DEP-3.10) |
| `usable` | `false` for an element E0 marked unusable (E0-4.1) |
| `query_error_code` | Present only when `usable` is `false`: the code a query that reads the element gets (E0-4.1) |
| `model_inputs` | Model elements it depends on (DEP-3.4 to DEP-3.8), without itself, written and ordered as in PLN-4.4, with `dataset` as a fourth kind before `field` |
| `physical_inputs` | Catalog columns it depends on, written and ordered as in PLN-4.5 |
| `security_policies` | Row policies that restrict the rows it reads, written and ordered as in PLN-4.7 |

**Model diff** (`menura.model_diff`):

| Field | Content |
|---|---|
| `producer` | The compiler build |
| `old_model_hash`, `new_model_hash` | The `model_hash` of each compared document |
| `changes[]` | `{object, change}`, `change` being `added`, `removed` or `changed` (DEP-4) |

## DEP-3 Rules

- **DEP-3.1** **Audience.** `dependencies` is a pure function of the handle's model. It needs no security context and reads no catalog state. It lists every element, authorized or not, so it is for modelers, developers and governance tools (ARC-6.2), never for end users. It contains no data values, attribute values or row-policy conditions: a policy appears only by name (SCY-4a.1).
- **DEP-3.2** **Elements.** Every dataset of the model is listed, summaries included, with every relationship, field, model or dataset metric, and row policy. Each list is ordered by `object` in CON-5 order of the normalized names (for a field, a policy or a dataset metric, the dataset name, then its own name; a model metric before the dataset metrics with the same first name part). The same model gives the same document, byte for byte after canonical JSON (P3).
- **DEP-3.3** **Unusable elements.** An unusable element is listed with `usable: false` and `query_error_code`, the code a query that reads it gets (E2-5 row 12a). Its inputs are computed as for a usable element, over the references that resolve. A policy has the usability of its dataset.
- **DEP-3.4** **Visibility inputs.** The visibility inputs of a dataset D are the relationships that the computation of V(D) follows (E4-6.5), the datasets they reach whose policies take part in V(D), the fields read by the conditions of those policies and of D's own policies, and the join-key fields of those relationships, each field with the fields it reads, directly or through other fields. They are computed for every user: roles and exemptions are not evaluated.
- **DEP-3.5** **Dataset.** `model_inputs` holds the visibility inputs of the dataset. A summary also holds its base dataset (`aggregate.of`), the visibility inputs of the base, and the fields named by its dimensions' `base` and read by its measures' `base_aggregate`, `FILTER` included, directly or through other fields. `physical_inputs` holds the columns read by those fields. `security_policies` lists the policies that take part in V(D); for a summary, those of V(base), which E5-3.7 applies to it.
- **DEP-3.6** **Field.** `model_inputs` holds its dataset, the fields its expression reads, directly or through other fields, and the `model_inputs` of its dataset. `physical_inputs` holds the columns of all of them; `security_policies` is that of its dataset.
- **DEP-3.7** **Metric.** `model_inputs` holds every metric it reads, transitively (derived inputs and context bases), and every field those metrics read, directly or through other fields: in expressions and `FILTER` conditions, a context metric's `time_dimension` and `remove` fields, and a semi-additive metric's `over`, `entity` and population fields; with the `model_inputs` of each such field. `physical_inputs` and `security_policies` are the unions of those of these fields.
- **DEP-3.8** **Relationship and policy.** A relationship's `model_inputs` holds its join-key fields with their `model_inputs`, and `physical_inputs` their columns; its `security_policies` is empty. A policy's `model_inputs` holds its dataset and the fields its condition reads, directly or through other fields, without the `model_inputs` of those fields; `physical_inputs` their columns; its `security_policies` is empty. A policy's dependents are found through `security_policies`, whose entry `<dataset>.<policy>` names the object `policy:<dataset>.<policy>`; `model_inputs` never holds a policy.
- **DEP-3.9** **No query content.** Nothing in the document depends on a query: paths chosen by a query, its dimensions and filters, and the summaries a plan may use are not dependencies. A plan's lineage (PLN-4) gives them for one query.
- **DEP-3.10** **Definition hash.** The definition hash of a dataset, relationship, field, metric or row policy is a hash (E0H-3.1) of that element's own normalized tree: for a field, its expression and logical type; for a metric, its expression and extensions; for a dataset, its `source`, keys, `access` and extensions other than its fields, metrics and policies; for a relationship, its datasets and keys; for a policy, its condition, roles and exemptions. It never covers labels, descriptions, `ai_context`, another element's definition, or the element's position in the document. An unusable element's hash also covers its own query error codes, but not those propagated from the elements it reads (E0-4.2a), so an upstream change is not reported as a change of its dependents.

## DEP-4 Diff

- **DEP-4.1** `diff(old, new)` is a pure function of two model-dependency documents as `dependencies` produces them. It reads only `model_hash` and the `object` and `definition_hash` of each entry.
- **DEP-4.2** An `object` only in `new` is `added`; only in `old`, `removed`; in both with a different `definition_hash`, `changed`. A renamed element is removed and added. An element whose inputs changed but whose own definition did not is not listed: the dependency graph gives its impact.
- **DEP-4.3** `changes` is ordered by kind (`dataset`, `relationship`, `field`, `metric`, `policy`), then by `object` as in DEP-3.2. Two documents of the same model give `changes: []`.
- **DEP-4.4** The CLI command `menura diff` loads the two models, computes their dependencies and returns their diff (ARC-5.6).
