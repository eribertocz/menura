# 0001 Security context admission and fingerprint

## Context

The 0.3.0 audit of the security context (issue #224) found four observable gaps: the order of
checks when a document is not JSON or lacks `contract`; how each declared attribute type is
checked; whether undeclared roles enter the fingerprint and how list values are sorted in it;
and the location of `WQRY-01` for a security context.

## Decision

- The checks follow the catalog order of CAT-9.1, then the declaration checks (SEC-3.8).
- Each declared type has exactly one accepted encoding; `integer` is signed 64-bit; scalars and
  lists are never promoted into each other (SEC-3.3).
- The fingerprint holds only declared roles and declared attributes; list values are sorted by
  value in their declared type (SEC-3.2, SEC-3.6).
- `WQRY-01` for a security context is located at `/contract`, reported only for an admitted
  context, before every warning of the query (SEC-3.9).

## Consequences

One encoding per value makes the fingerprint unique for equal contexts. Dropping undeclared
roles lets users that differ only in roles unknown to the model share cache entries; this is
safe because such roles grant nothing.

## Rules changed

SEC-3.2, SEC-3.3, SEC-3.6, SEC-3.8 (new), SEC-3.9 (new), E2-5 row 0a.
