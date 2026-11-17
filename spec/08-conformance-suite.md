# 8. Conformance suite

The suite is the executable form of this specification. A rule is implemented when its cases pass.

## CS-1 Layout

```
conformance/
  models/<model-name>/model.yaml           Apache Ossie model
  models/<model-name>/catalog-schema.json
  models/<model-name>/catalog-state.json   optional
  models/<model-name>/data.sql             DuckDB script that creates and fills the tables
  models/<model-name>/contexts/*.json      security contexts
  ossie/expected-failures.yaml             Apache Ossie suite cases not yet passing (OSS-2.4)
  cases/<area>/<case-id>.yaml              one case per file
  ossie/                                   Apache Ossie suites, vendored at the tracked commit
```

`<area>` is the rule prefix in lower case: `e0`, `e1`, …, `cat`, `qry`, `sec`, `add`, `ctx`, …

## CS-2 Case format

```yaml
id: e3-ambiguous-path-001
rules: [E3-3.2]                 # every rule ID this case covers
model: stores                   # folder under models/
security_context: analyst.json  # required for error, plan and runtime_error; under models/<model-name>/contexts/
query: { ... }                  # query document, or omitted for load-only cases
load_options: { ossie_format: development, model_name: shop }  # optional (CS-2.7)
expect:
  outcome: error                # load_error | error | plan | runtime_error
  error: E_AMBIGUOUS_PATH       # the first error code (E2-5.2)
  warnings: [W0F-01]            # optional: load warnings that MUST be reported, by code
  sql: expected.sql             # optional golden file, next to the case
  rows: expected.csv            # optional result after executing on DuckDB
  row_order: any                # any | exact
  tolerance: 1.0e-9             # for float columns only
```

| ID | Rule |
|---|---|
| CS-2.1 | `rules` lists at least one rule ID. CI fails if a rule ID of the coverage scope (CS-2.6) has no case (T1). |
| CS-2.2 | `outcome: plan` cases MUST have `rows`, so the generated SQL is executed, not only compared. |
| CS-2.3 | A golden `sql` file is compared byte for byte (P3). Regenerating golden files requires a pull request that touches only golden files and states why. |
| CS-2.4 | `rows` are compared with NULLs equal, decimals exactly, floats within `tolerance` (relative), and the row order as declared. |
| CS-2.5 | Runtime-policy cases expect `outcome: runtime_error` with the runtime code (`RADD-01`, …). |
| CS-2.6 | The **coverage scope** is the list of specification files in `spec/coverage-scope.txt`, one path per line; lines starting with `#` are comments. `rule-coverage` fails when a rule ID defined in a listed file is not covered (T1), and only reports the rule IDs of other files. The specification session adds the files of a version at its gate (7.1a); a file is never removed, so later work cannot lose coverage. |
| CS-2.7 | `load_options` holds load options of architecture section 3.5 (`ossie_format`, `model_name`, `namespace`, `grant_authenticated_to_all`); an option it omits takes its default. It lets a case cover a rule that applies only with a load option. |

## CS-3 Apache Ossie suites

- **CS-3.1** The Apache Ossie conformance suites are vendored under `conformance/ossie/` at the tracked development commit and run unchanged.
- **CS-3.2** Updating the commit is done only by the `ossie-update` skill (OSS-4.6), in its own pull request.

## CS-4 Running

- **CS-4.1** `just conformance` runs every case of `conformance/cases/` and checks the layout of `conformance/ossie/` (the stale-list rule of OSS-2.4). `just conformance <area>` runs one area. The outcomes of the Apache Ossie suites are judged only by `just conformance ossie`, the `ossie-suites` check (OSS-2.3).
- **CS-4.2** Each case runs in a fresh in-memory DuckDB database. For each source with three parts (`catalog.schema.table`), the runner attaches an in-memory database named after the first part before running `data.sql`; two-part sources (`schema.table`) live in the default database, and `data.sql` creates their schema. This applies to the compiler's own cases and to the Apache Ossie suites (CS-4.4). The session uses non-default settings (`TimeZone` not UTC, `integer_division = true`) to prove E6-4.10. The runner's DuckDB build includes the `icu` extension, which provides time zones, so no setting or query ever downloads an extension at run time.
- **CS-4.3** The runner reports, per rule ID, the cases that cover it and their result.
- **CS-4.4** For Apache Ossie suites, the runner loads the fixture with `ossie_format: development` and `grant_authenticated_to_all: true`, builds the catalog schema by reading the created tables, wraps each `query.json` in a query envelope with `time_zone: UTC`, and compares the first error code or the rows. SQL-form cases refer to relations as `<model name>.<dataset name>` (QRY-5.1).
- **CS-4.5** Runner security context for Apache Ossie suites: `{"contract": "semantic_compiler.security_context", "contract_version": "1.0.0", "subject": "conformance-runner", "roles": [], "attributes": {}}`, stored as `conformance/ossie/runner-context.json`.
- **CS-4.6** Layer 2 cases: the runner wraps the text of `query.ossie.sql` in the `sql` field of a query envelope (`time_zone: UTC`), runs `expected.sql` on the fixture to obtain the expected rows, and compares them under the case's comparison rules; an expected error is compared by its first code.

## CS-5 Producer suite

Architecture 3.3, item 5: the owner of a contract publishes a suite that its producers run in
their own CI. For the catalog contracts it is a set of documents with their expected outcome.

- **CS-5.1** Each case is a folder `conformance/producers/catalog/<case-id>/` with `document.json` (a catalog schema or state document), `schema.json` for a state case (the schema document it is checked against), and `expected.yaml`: `outcome` (`accepted`, `rejected` or `ignored`) and, unless accepted, the first `code` (CAT-9).
- **CS-5.2** The suite has at least one case per code `ECAT-01`–`ECAT-03` and `WCAT-01`–`WCAT-04`, one per CAT-8 rule that applies to the document kind, and accepted cases for a partitioned relation, a non-partitioned relation and a summary build.
- **CS-5.3** The compiler's CI runs every case through its public catalog admission and checks the outcome and the first code (CAT-9.1).
- **CS-5.4** Producers take the suite from this repository at the release tag they depend on. A document suite cannot prove the producer guarantees of CAT-7.1, which compare successive publications; each producer tests those itself.
