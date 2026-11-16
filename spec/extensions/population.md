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

## POP-4 Validation

| Code | Condition |
|---|---|
| `E0C-20` | Duplicate population name |
| `E0C-21` | Missing key for the declared `mode`, or a key of the other mode |
| `E0C-22` | A referenced field does not exist in the dataset |
| `E0C-23` | `required_when` is not a valid boolean expression over the dataset |
| `E0C-24` | `valid_from` or `valid_to` is not of kind `date` or `timestamp` |

## POP-5 Integrity checks

`validity`: `valid_from` ≤ `valid_to` when `valid_to` is not NULL; no overlapping intervals per
entity. `periodic`: one row per entity and period.
