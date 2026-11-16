# Glossary

| Term | Meaning |
|---|---|
| Anchor | In a query without metrics, the dimension dataset from which every other dimension dataset is reachable (E3-5) |
| Branch | The components of a query that share a home dataset, evaluated together (E3-4) |
| Catalog | The documents that describe the physical database: schema and state (CAT) |
| Compiled plan | The output of compilation: SQL, parameters, result schema, cache key, lineage, explanation (PLN) |
| Component | One aggregate inside a metric expression, evaluated over its home dataset (E0G-1) |
| Context modifier | A change to the context in which a metric is evaluated: period to date, prior period, share of total (CTX) |
| Empty-input value | The value of a component over no rows: NULL for `SUM`, 0 for `COUNT` (E4-5) |
| Evidence | A passing check recorded in the catalog state for the current data versions (CAT-7) |
| Home dataset | The single dataset a component's arguments read (E0G-1.1) |
| Population | The entities that should have a row in each period (POP) |
| Recomposable | A metric whose value can be rebuilt exactly from partial results (E0G-3) |
| Row identity | An identity of each home row, synthesized in the generated SQL, used to count each row once per group (E4-4) |
| Row policy | A rule that restricts which rows of a dataset a user sees (SCY-4, E4-6) |
| Semi-additive | A metric that is summed across some dimensions but not across `over` (ADD) |
| Stitching | Combining branches by dimension keys (E4-5) |
| Summary | A precomputed aggregate of a base dataset, used only when the result is identical (SUM) |
| Token | An opaque data version `{origin, snapshot}`, compared by equality only (CAT-6) |
