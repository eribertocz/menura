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
{"row_access": [{"name": "region_scope", "condition": "region IN USER_ATTRIBUTE('regions')", "roles": ["analyst"]},
                {"name": "open_scope", "condition": "TRUE", "roles": ["admin"]}]}
```

| Key | Required | Meaning |
|---|---|---|
| `name` | Yes | Unique in the model |
| `condition` | Yes | Boolean expression over the dataset's fields (no prefix) and `USER_ATTRIBUTE('name')` |
| `roles` | No | Roles the policy grants rows to; absent means it restricts everyone |
| `exempt_roles` | No | Roles the policy does not apply to |

- **SCY-4.1** A list attribute is used only with `IN` / `NOT IN`.
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
| `E0C-43` | `mode: authenticated` with `roles` |
| `E0C-44` | `mode: internal` on a dataset that is not a summary |
| `E0C-45` | Duplicate policy name |
| `E0C-46` | Invalid condition, or an undeclared attribute |
| `E0C-47` | Undeclared role in `roles`, `exempt_roles` or `fields` |
| `E0C-48` | List attribute used outside `IN` / `NOT IN` |
| `E0C-49` | `fields` names a missing field, or a role without dataset access |
