# Contract: model description

**Contract version:** 1.1.0 · **Owner:** `menura` · **Producer:** compiler (`describe`) · **Consumers:** hosts, translators, editors

## DSC-1 Purpose

`describe(handle, security_context)` tells a consumer what **this user** may ask: which
datasets, fields and metrics exist for them, how they can be grouped, and which paths need a
choice. Interfaces build their menus from it; translators propose only what it lists.

## DSC-2 Content

| Field | Content |
|---|---|
| `model_hash`, `security_fingerprint` | Identify the model and the context the description is valid for |
| `datasets[]` | Authorized datasets, each with its authorized fields: reference, label, description, logical type, `is_time`, the time grains it accepts, `ai_context` |
| `metrics[]` | Authorized metrics: reference, description, result type, `kind` (`base`, `derived`, `context`), and the datasets reachable from it (E3-3.4) |
| `ambiguous_paths[]` | Pairs of datasets with more than one path, with exactly two witness paths each, so a consumer can ask the user to choose (the full set is never enumerated) |

## DSC-3 Rules

- **DSC-3.1** Only objects the security context is authorized to use are listed (E2-4). An unauthorized object is absent, never marked as restricted.
- **DSC-3.2** Summaries (`access: internal`) are never listed.
- **DSC-3.2a** `reachable_datasets` and `ambiguous_paths` are computed over the authorized subgraph (E2-4.8).
- **DSC-3.3** No data values, attribute values or row-policy conditions appear.
- **DSC-3.4** Lists are ordered by normalized name (P3); the same inputs give the same document.
- **DSC-3.5** `describe` is a pure function of the model and the security context; it needs no catalog state.

Schema: [`schemas/model-description.schema.json`](schemas/model-description.schema.json) · Example: [`examples/model-description.example.json`](examples/model-description.example.json)
