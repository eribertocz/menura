# Extension: summaries

**Key:** `aggregate` · **Attached to:** Dataset

## SUM-1 Principle

A summary is a precomputed aggregate of a base dataset. E5 uses it instead of the base only if
the result is **exactly** the same. A summary never changes a number and never widens what a
user can see. When in doubt, the base is used.

## SUM-2 Declaration

A summary is a normal Apache Ossie dataset (its `source` and `fields` are validated like any
other) with `access: {"mode": "internal"}` and:

| Key | Required | Meaning |
|---|---|---|
| `of` | Yes | Base dataset; never another summary |
| `dimensions` | Yes (min. 1) | `{field, base, grain?}`: summary field, base field, and grain if the base field is a time field |
| `measures` | Yes (min. 1) | `{field, base_aggregate}`: summary field and the aggregate that produced it, e.g. `SUM(invoices.amount)` |
| `coverage_field` | Yes | Time field of the summary whose coverage the catalog reports |

- **SUM-2.1** `base_aggregate` is a single `SUM`, `COUNT(x)`, `MIN` or `MAX` call over base fields, optionally with `FILTER (WHERE …)` or a `CASE` argument.

## SUM-3 Conditions for use

| ID | Condition |
|---|---|
| SUM-3.1 | **Equivalence.** Every component of the metric matches a summary measure after normalization, and recomposes exactly, preserving its empty-input value: `SUM(x)`→`SUM(s)`; `COUNT(x)`→`COALESCE(SUM(c), 0)`; `MIN`→`MIN`; `MAX`→`MAX`; `AVG(x)`→`SUM(s) / NULLIF(COALESCE(SUM(c), 0), 0)`, where `s` is `SUM(x)` and `c` is `COUNT(x)` of the same argument and filter. Holistic aggregates never use summaries. |
| SUM-3.2 | **Exclusions.** Metrics whose `additivity` declaration is `non_additive` or `semi_additive`, branches that still have a `Deduplicate` after E5-3.2, and components whose argument is a float (float addition is not associative, so a recomposed sum can differ from the base) never use summaries in version 1. A metric deduced non-additive only because of an `AVG` component (E0G-4) is not excluded: `AVG` recomposes exactly (SUM-3.1); holistic components are excluded by SUM-3.1. |
| SUM-3.2a | **Values that depend on the query.** A summary is not used if a dimension `base` field or a field a `base_aggregate` reads uses `CURRENT_DATE`, `CURRENT_TIME` or `CURRENT_TIMESTAMP`, directly or through other fields, or reads a `timestamp` with time zone other than as the plain column of a time dimension (CAT-7.2, condition 5): its rows would depend on the time or time zone of the build. |
| SUM-3.3 | **Grain.** Every query dimension, filter field and `date_range` field is a summary dimension with the same `base`, or is reached from the base through a relationship whose `from_columns` the summary keeps. |
| SUM-3.4 | **Time grain.** The query grain derives from the summary grain by exact grouping: `day` → all; `week` → `week`; `month` → `month`, `quarter`, `year`; `quarter` → `quarter`, `year`; `year` → `year`. |
| SUM-3.3a | **Truncated fields.** A summary dimension with `grain` holds truncated values, so it serves only a query dimension whose time grain derives from it (SUM-3.4) and a date range aligned on its grain (SUM-3.5). A query dimension without a time grain, a query filter, an applicable row-policy condition, an `order_by` that is not an output column, or a `from_columns` that reads such a field prevents the use of the summary. |
| SUM-3.5 | **Time filter alignment.** A date range on a time dimension of the summary starts on the first day of a period of that dimension's grain and ends on the last day of one (or is open-ended at `9999-12-31`). A relative range is aligned after it is resolved to an absolute range (QRY-4.2). |
| SUM-3.6 | **Policy preservation.** The user has access to the base; every applicable policy of the base can be applied to summary columns with the same `base`; every propagated policy has its relationship columns kept. A summary declares no policies of its own. |
| SUM-3.7 | **Freshness.** All conditions of CAT-7.2, including the time zone. |
| SUM-3.8 | **Same type.** A recomposed component returns the logical type of the base component (E0F-5); E6 casts it (E6-4.15). `COUNT(x)` recomposed as `COALESCE(SUM(c), 0)` is a 64-bit integer. |

## SUM-4 Visibility

- **SUM-4.1** A summary is never named by a query and never offered as a dimension.
- **SUM-4.2** Its physical relation MUST be readable only by the authorized executor's identity. The compiler cannot enforce this (P2); it is a deployment requirement, and plans that use a summary say so in the explanation.

## SUM-5 Integrity check

For each summary, E0 generates a query that recomputes it from the base within its coverage and
compares row by row (E0H-4).

## SUM-6 Validation

| Code | Condition |
|---|---|
| `E0C-50` | `of` missing or is a summary |
| `E0C-51` | A `field` or `base` does not exist |
| `E0C-52` | Invalid `base_aggregate` |
| `E0C-53` | Time dimension without `grain`, or `grain` on a non-time field |
| `E0C-54` | `coverage_field` is not a time dimension of the summary |
| `E0C-55` | Summary declares `row_access`, or at least one dataset metric (development format; an empty `metrics` list declares none) |
| `E0C-56` | Summary `access` is not `internal` |
| `E0C-57` | A summary field's logical kind differs from its base: the base field's for a dimension, the `base_aggregate` result type for a measure |

### SUM-6.1 Validation details

| ID | Rule |
|---|---|
| SUM-6.1.1 | **Shape, in E0.c.** `aggregate` is an object with exactly `of` (a string), `dimensions` (a non-empty list of objects with `field` and `base`, strings, and an optional `grain`), `measures` (a non-empty list of objects with exactly `field` and `base_aggregate`, strings) and `coverage_field` (a string). `grain` is `day`, `week`, `month`, `quarter` or `year`. A missing `of` is `E0C-50`. Any other missing key, wrong type or value is `E0C-03`, and an unknown key is `E0C-02`. A dataset is a summary when its entry of ours has the key `aggregate`, valid or not (E0E-3.6). |
| SUM-6.1.2 | **Names, in E0.d.** `of` names a dataset, `field` and `coverage_field` name fields of the summary, and `base` names a field of the `of` dataset, all as declared names (E0D-1.4) compared under E0D-1.3. `of` with no such dataset, or naming a summary (itself included), is `E0C-50`. A `field` or `base` with no such field, a summary field named by no dimension and no measure or by more than one, and a field reference in `base_aggregate` that does not exist, are `E0C-51`. `coverage_field` must be the `field` of a dimension that has `grain` (`E0C-54`); a `coverage_field` that names no field of the summary is only `E0C-54`, not also `E0C-51`. When `of` is `E0C-50`, the `base` names and the `base_aggregate` references are not checked (there is no dataset to check them against). In E0.d, a qualified `base_aggregate` reference to a missing dataset, to a summary, or to a missing field is `E0C-51`; an unqualified reference, a reference to another existing dataset, and a text that does not parse are left to E0.f (`E0C-52`). |
| SUM-6.1.3 | **`base_aggregate`, in E0.f (`E0C-52`).** It is parsed with the expression grammar (E0F-9) as a model metric expression whose references are `of.field`. It is exactly one call of `SUM`, `COUNT(x)`, `MIN` or `MAX`, with no `DISTINCT` and no `COUNT(*)`, and with an optional `FILTER (WHERE …)`. Its argument and filter are row-level expressions (a `CASE` is allowed) that read only the `of` dataset. Anything else is `E0C-52`. A `base_aggregate` of that shape that does not type under E0F (for example `SUM` over a text field) is also `E0C-52`, never an `E0F-*` code; as POP-4.5 states, this is checked only when every field it reads has a type. A measure with `E0C-52` is not checked by SUM-6.1.5. |
| SUM-6.1.4 | **Time dimensions, in E0.f (`E0C-53`).** A dimension has `grain` exactly when its `base` field has kind `date` or `timestamp`. This is checked only when E0.f typed the `base` field. |
| SUM-6.1.5 | **Kinds, in E0.f (`E0C-57`).** A dimension's summary field has the logical kind of its `base` field, and a measure's summary field has the kind of the `base_aggregate` result type (E0F-5), compared under E0F-4.1. This is checked only on typed fields. |
| SUM-6.1.6 | **Access (`E0C-55`, `E0C-56`).** A summary without `access`, or with a valid mode (SCY-3) other than `internal`, is `E0C-56`. A missing or invalid `mode` is only `E0C-41` (SCY-5.1.3), not also `E0C-56`. A summary never gets `W0C-41`, and `grant_authenticated_to_all` does not apply to it. |
| SUM-6.1.7 | **Visibility at load (SUM-4.1).** A summary is not a dataset that other model elements may name: a metric that refers to it is treated as referring to a missing dataset (`E0D-04`), and E0.f treats that reference as missing too (no type, no `E0F-*` diagnostic for it), an extension reference to it gets the extension's own code (E0D-3.2), and a relationship to or from it is `E0E-02` (E0E-3.6), not `E0D-05`. This holds whatever E0.c reported: a dataset is a summary by the rule of SUM-6.1.1. |
| SUM-6.1.8 | **Catalog state (CAT-8.8).** The check that `aggregate_build` appears only on summaries, with periods of the grain of the summary's coverage dimension, needs the summaries of the loaded model. It is done by `load` and `with_state` with the internal model (E0H). |
