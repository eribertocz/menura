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
| `object` | `dataset:<name>`, `relationship:<name>`, `field:<dataset>.<field>`, `metric:<name>` or `policy:<dataset>.<policy>`, with normalized names (E0H-1.1, PLN-7.2) |
| `kind` | Metrics only: `base`, `derived` or `context`, as in the model description (DSC-2) |
| `definition_hash` | The element's definition hash (DEP-3.10) |
| `usable` | `false` for an element E0 marked unusable (E0-4.1) |
| `query_error_code` | Present only when `usable` is `false`: the code a query that reads the element gets (E0-4.1) |
| `model_inputs` | Model elements it depends on (DEP-3.4 to DEP-3.8), written and ordered as in PLN-4.4, with `dataset` as a fourth kind before `field` |
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
- **DEP-3.2** **Elements.** Every dataset of the model is listed, summaries included, with every relationship, field, model or dataset metric, and row policy. Each list is ordered by `object` in CON-5 order of the normalized names (for a field or a policy, the dataset name, then its own name). The same model gives the same document, byte for byte after canonical JSON (P3).
- **DEP-3.3** **Unusable elements.** An unusable element is listed with `usable: false`, its `query_error_code`, and empty `model_inputs`, `physical_inputs` and `security_policies`. A usable element whose inputs include an unusable one does not exist: E0 propagates unusability (E0-4.2a).
- **DEP-3.4** **Dataset.** `model_inputs` is empty, except for a summary: its base dataset (`aggregate.of`) and every field its dimensions and measures read (SUM-2). `physical_inputs` is empty. `security_policies` lists every row policy that takes part in the visible rows V(D) of the dataset (E4-6.5), for every user: roles and exemptions are not evaluated.
- **DEP-3.5** **Relationship.** `model_inputs` holds the fields of its join keys and their datasets; `physical_inputs` the columns those fields read. `security_policies` is empty.
- **DEP-3.6** **Field.** `model_inputs` holds its dataset and every field its expression reads, directly or through other fields. `physical_inputs` holds the columns those expressions read. `security_policies` is that of its dataset (DEP-3.4).
- **DEP-3.7** **Metric.** `model_inputs` holds every metric it reads, transitively (derived inputs and context bases); every field those metrics read, including `FILTER` conditions, a context metric's `time_dimension` and `remove` fields, and a semi-additive metric's `over`, `entity` and population fields; the home datasets of each; and every relationship that row-policy propagation follows from those home datasets (E4-6.5). `physical_inputs` holds the columns read by those fields, by the conditions of the policies in `security_policies`, and by the join keys of those relationships. `security_policies` is the union of the `security_policies` of its home datasets and of the population datasets it reads.
- **DEP-3.8** **Policy.** `model_inputs` holds its dataset and the fields its condition reads; `physical_inputs` the columns those fields read; `security_policies` is empty. Its `definition_hash` changes when its condition, roles or exemptions change, without revealing them.
- **DEP-3.9** **No query content.** Nothing in the document depends on a query: paths chosen by a query, its dimensions and filters, and the summaries a plan may use are not dependencies. A plan's lineage (PLN-4) gives them for one query.
- **DEP-3.10** **Definition hash.** The definition hash of a dataset, relationship, field, metric or row policy is a hash (E0H-3.1) of that element's own normalized tree: for a field, its expression and logical type; for a metric, its expression and extensions; for a dataset, its `source`, keys, `access` and extensions other than its fields, metrics and policies; for a relationship, its datasets, keys and cardinality; for a policy, its condition, roles and exemptions. It never covers labels, descriptions, `ai_context`, another element's definition, or the element's position in the document. An unusable element's hash also covers its query error codes.

## DEP-4 Diff

- **DEP-4.1** `diff(old, new)` is a pure function of two model-dependency documents as `dependencies` produces them. It reads only `model_hash` and the `object` and `definition_hash` of each entry.
- **DEP-4.2** An `object` only in `new` is `added`; only in `old`, `removed`; in both with a different `definition_hash`, `changed`. A renamed element is removed and added. An element whose inputs changed but whose own definition did not is not listed: the dependency graph gives its impact.
- **DEP-4.3** `changes` is ordered by kind (`dataset`, `relationship`, `field`, `metric`, `policy`), then by `object` as in DEP-3.2. Two documents of the same model give `changes: []`.
- **DEP-4.4** The CLI command `menura diff` loads the two models, computes their dependencies and returns their diff (ARC-5.6).
