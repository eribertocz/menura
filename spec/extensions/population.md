# Extension: population

**Key:** `populations` · **Attached to:** Dataset

## POP-1 Purpose

Declare which entities **should** have a row in each period. It is the only source the compiler
uses to decide that an entity is missing (ADD-4.2).

## POP-2 Schema

`populations` is a list, declared on the dataset that describes the entities. Each item:

| Key | Required | Meaning |
|---|---|---|
| `name` | Yes | Unique in the model |
| `mode` | Yes | `validity` (validity intervals) or `periodic` (one row per entity and period) |
| `entity` | Yes | Fields of the dataset that identify the entity |
| `valid_from` | If `validity` | Start of validity, inclusive |
| `valid_to` | If `validity` | End of validity, inclusive; NULL means open-ended |
| `partial_period` | If `validity` | `any_day`, `full_period` or `period_end` |
| `period` | If `periodic` | Period field of the row |
| `grain` | If `periodic` | Grain of `period` |
| `required_when` | No | Boolean expression over the dataset's fields; the entity is expected only if it holds |

Fields are named without dataset prefix: they belong to the dataset that declares the extension.

## POP-3 Meaning

| ID | Rule |
|---|---|
| POP-3.1 | `validity`: an entity is expected in a period if its validity covers the period according to `partial_period`: at least one day (`any_day`), every day (`full_period`), or the last day (`period_end`). It is evaluated at the `snapshot_grain` of the metric that uses it. |
| POP-3.2 | `periodic`: an entity is expected in a period if its row for that period exists. |
| POP-3.3 | In both modes, `required_when` MUST also hold. |
| POP-3.4 | In `validity`, an entity may have several non-overlapping validity rows (deactivation and reactivation). |
| POP-3.5 | **Days and periods.** A `timestamp` value belongs to its day in the query `time_zone`, truncated as E6-5.2 states; a `date` value is its own day. For a period with first day `p` and last day `l` at the grain: `any_day` holds when `valid_from` ≤ `l` and (`valid_to` is NULL or `valid_to` ≥ `p`); `full_period` when `valid_from` ≤ `p` and (`valid_to` is NULL or `valid_to` ≥ `l`); `period_end` when `valid_from` ≤ `l` and (`valid_to` is NULL or `valid_to` ≥ `l`). A NULL `valid_from` is never expected. |
| POP-3.6 | **Periodic rows.** In `periodic`, a row is for period `p` when its `period`, truncated to `grain` with POP-3.5, equals `p`. A NULL `period` is never expected. |

## POP-4 Validation

| Code | Condition |
|---|---|
| `E0C-20` | Duplicate population name |
| `E0C-21` | Missing key for the declared `mode`, or a key of the other mode |
| `E0C-22` | A referenced field does not exist in the dataset |
| `E0C-23` | `required_when` is not a valid boolean expression over the dataset |
| `E0C-24` | `valid_from`, `valid_to` or `period` is not of kind `date` or `timestamp` |

- **POP-4.1** `populations` is a list of objects. Every item has `name`, `mode` and `entity`, and the keys of its `mode`; a missing one, or a key of the other mode, is `E0C-21`. A key that POP-2 does not define is `E0C-02` (E0C-2.4).
- **POP-4.2** Values: `name` is a non-empty string; `mode` is `validity` or `periodic`; `entity` is a non-empty list of field names without repeats; `valid_from`, `valid_to` and `period` are field names; `partial_period` is `any_day`, `full_period` or `period_end`; `grain` is `day`, `week`, `month`, `quarter` or `year`; `required_when` is a string. Any other type or value is `E0C-03`.
- **POP-4.3** A field name is read as a declared name (E0D-1.4) and names a field of the declaring dataset under E0D-1.3; a name with no such field, in any key or inside `required_when`, is `E0C-22`. Population names are compared the same way across the whole model, and a repeat is `E0C-20` at the later entry.
- **POP-4.4** `required_when` is parsed with the expression grammar (E0F-9) as a row-level expression of the declaring dataset: a bare name is a field of that dataset (as in a dataset metric, E0D-3). A text that does not parse, a construct outside the subset, a qualified reference, an aggregate, or a type other than boolean is `E0C-23`.
- **POP-4.5** A field without a type already has its own diagnostic. The structural conditions of `E0C-23` are checked whatever the types of the fields read: a text that does not parse, a construct outside the subset, a qualified reference, an aggregate, `AGG(…)` or `USER_ATTRIBUTE(…)`. The boolean type of `required_when` is checked only when every field it reads has a type, and `E0C-24` only on fields that E0.f typed.
- **POP-4.6** Every diagnostic is located as E0C-2.6 states, and is reported in the substage the E0 code table assigns to its code.

## POP-5 Integrity checks

`validity`: `valid_from` ≤ `valid_to` when `valid_to` is not NULL; no overlapping intervals per
entity. `periodic`: one row per entity and period.
