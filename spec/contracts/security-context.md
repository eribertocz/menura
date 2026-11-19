# Contract: security context

**Contract version:** 1.1.0 · **Owner:** `menura` · **Producer:** host authentication · **Consumer:** compiler

## SEC-1 Purpose

The security context states who is asking. The compiler uses it to authorize datasets and fields
and to inject row filters. It never authenticates anyone: the host has already verified the
identity and is responsible for the context it sends.

## SEC-2 Document

| Field | Required | Rule |
|---|---|---|
| `contract` | Yes | `menura.security_context` |
| `contract_version` | Yes | Semantic version, accepted as in CAT-3.2 |
| `subject` | Yes | Opaque identifier of the user. Used only in the audit record, never in SQL or in authorization. |
| `roles` | Yes (may be empty) | Roles granted to the user |
| `attributes` | Yes (may be empty) | Attribute values that row policies read, as scalars or lists of scalars |

Schema: [`schemas/security-context.schema.json`](schemas/security-context.schema.json) · Example: [`examples/security-context.example.json`](examples/security-context.example.json)

## SEC-3 Rules

- **SEC-3.1** Every executable compilation requires a security context. There are no anonymous queries. Model validation (E0) needs none.
- **SEC-3.2a** An attribute not declared in the model's `security_context` is ignored and listed in the explanation; it is excluded from the fingerprint.
- **SEC-3.2** A role not declared in the model grants nothing. It is not an error, because identity providers carry roles unrelated to the model; the explanation lists it as ignored.
- **SEC-3.3** An attribute is checked against its declaration in the model's `security_context` and encoded as CON-4 requires (a `decimal` or `date` attribute is a canonical string; an `integer` attribute outside ±(2^53 − 1) is a string). A value of the wrong type or encoding is an error (`ESEC-01`).
- **SEC-3.4** If a row policy that applies to the query reads an attribute the context does not provide, compilation fails (`ESEC-02`). The compiler never substitutes a default value.
- **SEC-3.5** Attribute values reach SQL only as bound parameters, never as text.
- **SEC-3.6** The context fingerprint is the SHA-256 hash of the RFC 8785 canonical JSON of `{roles, attributes}` after CON-4 encoding, with `roles` and list attribute values sorted (CON-5 order) and duplicates removed. `subject` is excluded, so users with identical roles and attributes share cache entries safely: their results are identical by construction.
- **SEC-3.7** Authorization is deny by default (P9). How the model declares access, field access and row policies is specified in the security extension.

## SEC-4 Errors

| Code | Condition |
|---|---|
| `ESEC-01` | The document fails its JSON Schema, or an attribute value does not match its declared type or encoding |
| `ESEC-02` | An applicable row policy needs an attribute the context does not provide |
| `ESEC-03` | Unsupported `contract` or `contract_version` |
