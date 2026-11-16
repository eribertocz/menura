# Stages

The compiler has two moments: **load** a model once, and **compile** queries many times.

## STG-1 Load

| Stage | Input | Output |
|---|---|---|
| [E0 Load and validate](E0/README.md) | Model + catalog schema | Immutable validated model with its warnings, recorded integrity checks (generated later by `integrity_checks`) and summary definition hashes — or the errors found |

## STG-2 Compile

| Stage | Input | Output |
|---|---|---|
| [E1 Parse](E1-parse.md) | Query document (wide-table or SQL form) | Query tree |
| [E2 Bind](E2-bind.md) | Query tree + model + security context | Bound query: every name resolved and authorized |
| [E3 Paths](E3-paths.md) | Bound query | Join path for every metric component and filter |
| [E4 Plan](E4-plan.md) | Bound query + paths + security context | Logical plan with grain, context, filters and row policies applied |
| [E5 Optimize](E5-optimize.md) | Logical plan + catalog state + security context | Equivalent, cheaper logical plan (summary selection) |
| [E6 Generate](E6-generate.md) | Logical plan + dialect | Compiled plan |

Every output in these tables is a data type of a shared crate (S9): the E0 output in `model`, the E1–E5 outputs in `representations`.

## STG-3 Rules

- **STG-3.1** Each stage consumes only the output of the previous stages, never the model or query documents; the security context reaches E2, E4 and E5 already validated at the `compile` entry (E2-5 row 0a), and the catalog state reaches E5 already validated by `load`/`with_state` (P8).
- **STG-3.2** Row policies are injected in E4, before optimization, so that E5 can never choose a relation that bypasses them.
- **STG-3.3** Grain and context are resolved together in E4, because a context modifier (for example, percent of total) changes the grain.
- **STG-3.4** E0 reports every problem it finds in one pass: a substage whose inputs failed is skipped, but independent substages still run.
- **STG-3.5** A query error stops compilation at the stage that detects it, following the precedence of E2-5.
- **STG-3.6** Errors block; warnings inform. Every diagnostic has a stable code, an English message and a location (PLN-5).
