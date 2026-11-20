# 0002 E1 envelope, time zone names, references and the 0.3.0 SQL-form check

## Context

The 0.3.0 audit of E1 (issue #227) found observable gaps in the envelope locations, the
accepted spelling of `time_zone`, the syntax of references and output names, and the location
of `WQRY-01`. VS-2 also required the whole SQL-subset parser in 0.3.0, only to report errors.

## Decision

- Envelope checks follow CAT-9.1 and SEC-3.8; each member is located at its own pointer, and a
  member that cannot be read is left to the JSON Schema, except `contract` and
  `contract_version` (E1-2.2).
- `time_zone` is the exact spelling of a zone or link of the bundled IANA database (E1-2.3).
  The approved database is `jiff-tzdb`, used through `jiff` without system time-zone access.
- References are split at `.` outside delimited identifiers; E1 never rejects a reference, and
  E2 reports what names nothing as `E_NAME_NOT_FOUND` (E1-3.1).
- A query filter that does not parse is `E0F-01`, and one outside the subset `E0F-02`, both in E1
  right after the JSON Schema, `filter` before `filters` (E1-3.2, E2-5 row 1).
- Output names are taken as written, without the quotes of a delimited part (E1-5.1).
- `WQRY-01` is located at `/contract` or at the extension's namespace, after the security
  context's warning (E1-2.4).
- Maintainer decision: before 0.5.0 the SQL form is detected right after the JSON Schema check
  (VS-2), so 0.3.0 parses no SQL.

## Consequences

A malformed reference and an unknown one give the same code, so the error never depends on the
spelling of a name the user cannot see (PLN-5.2). Before 0.5.0, an SQL query that would raise
`EQRY-05` or `EQRY-13` returns `E_NOT_YET_SUPPORTED` instead.

## Rules changed

E1-2.2, E1-2.3, E1-2.4 (new), E1-3.1, E1-3.2, E1-5.1, E1-6, E2-5 row 1, VS-2.
