# Extension: security

**Keys:** `security_context` (model); `access`, `row_access` (dataset)

## SCY-1 Principle

Nothing is accessible without explicit authorization (P9). A dataset is never public because it
lacks a written policy. Security has two levels, and both always apply:

1. **Access:** may the user use the dataset, and which of its fields and metrics.
2. **Row policies:** which rows the user sees, once access is granted. A row policy never grants access.

## SCY-2 `security_context` (model, required)

```json
{"security_context": {"roles": ["admin", "analyst"],
                      "attributes": [{"name": "regions", "type": "list<text>"}, {"name": "area_id", "type": "integer"}]}}
```

- **SCY-2.1** `roles` and `attributes` are required (each may be empty). A model without `security_context` declares no roles or attributes (`W0C-40`).
- **SCY-2.2** Attribute types: `text`, `integer`, `decimal(p,s)` (precision and scale are required, e.g. `decimal(18,2)`), `date`, `boolean`, and `list<…>` of any of them.

## SCY-3 `access` (every dataset)

| `mode` | Meaning |
|---|---|
| `authenticated` | Any user with a security context. A declared opening, never an omission. |
| `roles` | Users holding at least one listed role. |
| `internal` | No consumer. Only valid on summaries; only the compiler selects them. |

```json
{"access": {"mode": "roles", "roles": ["analyst", "admin"],
            "fields": [{"names": ["unit_cost"], "roles": ["admin"]}]}}
```

- **SCY-3.0** A dataset without `access` is inaccessible (`W0C-41`), unless the load option `grant_authenticated_to_all: true` is set (DEV-4).
- **SCY-3.1** `fields` restricts named fields of the dataset to the listed roles, which MUST already have access to the dataset. A metric whose components read a restricted field is restricted the same way.
- **SCY-3.1a** A restriction protects **source columns**, not names: every field whose expression reads a column that a restricted field reads is restricted to the same roles (the intersection, if several restrictions apply).
- **SCY-3.2** Value masking is out of version 1.

## SCY-4 `row_access` (dataset, optional)

```json
{"row_access": [{"name": "region_scope", "condition": "region IN (USER_ATTRIBUTE('regions'))", "roles": ["analyst"]},
                {"name": "open_scope", "condition": "TRUE", "roles": ["admin"]}]}
```

| Key | Required | Meaning |
|---|---|---|
| `name` | Yes | Unique in the model |
| `condition` | Yes | Boolean expression over the dataset's fields (no prefix) and `USER_ATTRIBUTE('name')` |
| `roles` | No | Roles the policy grants rows to; absent means it restricts everyone |
| `exempt_roles` | No | Roles the policy does not apply to |

- **SCY-4.1** A list attribute is used only as the single item of an `IN` / `NOT IN` list, as in `region IN (USER_ATTRIBUTE('regions'))`, and the test holds for any of its values. The parentheses are required by the grammar (E0F-9.1).
- **SCY-4.2** No aggregates, subqueries or other datasets in a condition.
- **SCY-4.3** Semantics (combination, exemption, propagation, no grant) are in E4-6.

## SCY-4a Confidentiality of the compiled plan

- **SCY-4a.1** The compiled plan (its SQL, parameters and explanation) contains policy conditions and the names of datasets joined only to apply them. It is confidential to the host and the executor and MUST NOT be shown to the end user. What a user may see is the model description (DSC) and the result rows.

## SCY-5 Validation

| Code | Condition |
|---|---|
| `W0C-40` | The model has no `security_context` |
| `W0C-41` | A dataset has no `access`; it is inaccessible |
| `E0C-41` | `access` with an invalid `mode` |
| `E0C-42` | `mode: roles` with an empty or undeclared role list |
| `E0C-43` | `roles` with a mode other than `roles`, or `fields` with `mode: internal` |
| `E0C-44` | `mode: internal` on a dataset that is not a summary |
| `E0C-45` | Duplicate policy name |
| `E0C-46` | Invalid condition, or an undeclared attribute |
| `E0C-47` | Undeclared role in `roles`, `exempt_roles` or `fields` |
| `E0C-48` | List attribute used outside `IN` / `NOT IN` |
| `E0C-49` | `fields` names a missing field, or a role without dataset access |

### SCY-5.1 Validation details

| ID | Rule |
|---|---|
| SCY-5.1.1 | **Names.** Roles and attribute names are compared as exact strings, in the model and in the security context alike (SEC-3.2). Field names and policy names are declared names (E0D-1.4) compared under E0D-1.3; policy names are compared across the whole model (`E0C-45`). |
| SCY-5.1.2 | **`security_context`, in E0.c.** It is an object with exactly `roles` (a list of non-empty strings without repeats) and `attributes` (a list of objects with exactly `name`, a non-empty string, and `type`, with no name repeated). `type` is written as in SCY-2.2: `decimal(p,s)` with 1 ≤ p ≤ 38 and 0 ≤ s ≤ p, and `list<…>` of a non-list type. Any other shape or value is `E0C-03`, and an unknown key is `E0C-02`. |
| SCY-5.1.3 | **`access`, in E0.c.** A missing `mode` or a value outside SCY-3 is `E0C-41`. `roles` is required with `mode: roles` and allowed only there (`E0C-42`, `E0C-43`). `fields` is allowed with `authenticated` and `roles` (`E0C-43`). Each `fields` item has exactly a non-empty `names` list and a non-empty `roles` list. `access.roles` and every `names` and `roles` list are lists of non-empty strings without repeats. Any other shape is `E0C-03`. An undeclared role is `E0C-47`; when the roles of `security_context` cannot be read (SCY-5.1.2), no `E0C-47` is reported. |
| SCY-5.1.4 | **`fields`, in E0.d (`E0C-49`).** Each name is a field of the dataset. Each role has access to the dataset: it is listed in `access.roles`, or the mode is `authenticated`. |
| SCY-5.1.5 | **`row_access`, in E0.c.** It is a list of objects with `name` and `condition` (strings), and optional `roles` and `exempt_roles` (non-empty lists of strings without repeats). Any other shape is `E0C-03`, and an unknown key is `E0C-02`. A duplicate name is `E0C-45`, and an undeclared role is `E0C-47`. |
| SCY-5.1.6 | **Conditions, in E0.f.** `condition` is parsed with the expression grammar (E0F-9) as a row-level expression of the dataset: a bare name is a field of the dataset (as in a dataset metric, E0D-3), and `USER_ATTRIBUTE('name')` has the type its declaration gives. A text that does not parse, a construct outside the subset, a qualified reference, a missing field, an aggregate, `AGG(…)`, an undeclared attribute, or a type other than boolean is `E0C-46`. A list attribute anywhere else than SCY-4.1 allows is `E0C-48`. As POP-4.5 states for populations, the structural conditions are checked whatever the field types, and the boolean type only when every field read has a type. |
| SCY-5.1.7 | **Missing `access` (`W0C-41`).** It is located at the dataset. With `grant_authenticated_to_all: true` (DEV-4), the dataset is read as `mode: authenticated` without `fields`, and `W0C-41` is not reported. `W0C-40` is located at the semantic model. |
| SCY-5.1.8 | **Restrictions (SCY-3.1, SCY-3.1a).** E0 records, for each field, the set of roles it is restricted to, or none. A field that reads a source column read by a field named in a `fields` item gets the intersection of every applicable restriction. Restrictions do not chain: only the columns of the named fields are protected, so a field restricted through a shared column does not protect its other columns. A metric is restricted to the intersection of the restrictions of the fields its components read. A derived or context metric is restricted to the intersection of the restrictions of its inputs. The logic is in the `security` crate (S5). |
