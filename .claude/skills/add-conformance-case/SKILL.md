---
name: add-conformance-case
description: Add a conformance case in the exact format of spec/08-conformance-suite.md. Use when a rule needs a test case.
---

# Add a conformance case

1. Read `spec/08-conformance-suite.md` sections CS-1 and CS-2.
2. Reuse an existing model under `conformance/models/` if one fits; create a new one only if none does, with `model.yaml`, `catalog-schema.json` and `data.sql`.
3. Name the case `<area>-<short-description>-<nnn>` and place it in `conformance/cases/<area>/`.
4. List every rule ID the case covers in `rules`.
5. Set `expect.outcome`. For `plan`, always include `rows`; include `sql` only when the exact SQL text is itself the subject of the rule.
6. Keep data minimal: the fewest rows that prove the rule, including one row that would expose the wrong behavior (an unmatched key, a NULL, a duplicate).
7. Run `just conformance <area>` and confirm the case fails before the implementation and passes after.
