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
  differential/divergences.yaml            classified divergences of differential cases (CS-4.7)
  ossie/                                   Apache Ossie suites, vendored at the tracked commit
```

`<area>` is the rule prefix in lower case: `e0`, `e1`, …, `cat`, `qry`, `sec`, `add`, `ctx`, …. A prefix whose lower-case form Windows reserves as a file name (`con`, `prn`, `aux`, `nul`, `com1`–`com9`, `lpt1`–`lpt9`) takes its full word instead: the area of `CON` is `contract`. Every path in the repository must check out on Windows. During a version, the area `next` holds the cases that need the next slice, which MAY stay pending (VS-4); the issue that widens the slice moves them to the areas of their rule prefixes, where they must pass like any other case. `multi` is a normal area: its cases must pass.

## CS-2 Case format

```yaml
id: e3-ambiguous-path-001
rules: [E3-3.2]                 # every rule ID this case covers
model: stores                   # folder under models/
security_context: analyst.json  # required for error, plan, runtime_error and describe; under models/<model-name>/contexts/
query: { ... }                  # query document, or omitted for load-only cases
load_options: { ossie_format: development, model_name: shop }  # optional (CS-2.7)
compile_options: { actor: agent-7 }  # optional: only actor (CS-2.18)
expect:
  outcome: error                # load_error | error | plan | runtime_error | describe | integrity_checks
  error: E_AMBIGUOUS_PATH       # the first error code (E2-5.2)
  alternatives: ["metric:BILLED"]  # optional, error only: the first diagnostic's alternatives (CS-2.17)
  warnings: [W0F-01]            # optional: warnings that MUST be reported, by code (CS-2.4b)
  location: { document: model, pointer: /semantic_model/0/datasets/0, line: 7, column: 9 }  # optional: location of the first error
  sql: expected.sql             # optional golden file, next to the case
  rows: expected.csv            # optional result after executing on DuckDB
  row_order: any                # any | exact
  tolerance: 1.0e-9             # for float columns only
  lineage: expected.lineage.json            # optional golden file of the plan's lineage (CS-2.11)
  explanation: ["<code>", …]               # optional: the plan's explanation codes, in order (CS-2.12)
  audit: expected.audit.json                # optional golden audit record; compiles with audit: true (CS-2.13)
  cache_key: "<64 hex digits>"            # optional: the plan's cache key, 64 lowercase hex digits (CS-2.14)
  document: expected.describe.json          # describe and integrity_checks only: golden output document (CS-2.15)
oracle: expected.oracle.sql      # optional: reference SQL for a differential case (CS-4.7); requires outcome plan
```

| ID | Rule |
|---|---|
| CS-2.1 | `rules` lists at least one rule ID. CI fails if a rule ID of the coverage scope (CS-2.6) has no case (T1). |
| CS-2.2 | `outcome: plan` cases MUST have `rows`, so the generated SQL is executed, not only compared. |
| CS-2.3 | A golden `sql` file is compared byte for byte (P3). Regenerating golden files requires a pull request that touches only golden files and states why. |
| CS-2.3a | **ClickHouse.** From 1.1.0 every `outcome: plan` or `outcome: runtime_error` case also runs with `dialect: clickhouse` (E6-7.10): its golden ClickHouse SQL is the file named by `sql_clickhouse`, compared like `sql` (CS-2.3), and its result is the case's own, the same for both dialects: the `rows` of a `plan` case, the runtime code of a `runtime_error` case. A case whose query reads a relation or needs a value ClickHouse cannot express (E6-7.1, E6-7.12) sets `clickhouse: e6_01` instead of `sql_clickhouse`, and passes on ClickHouse only when compilation returns `E6-01`. So that cases run on both engines, the models of `plan` cases name relations with at most two parts (`shop.orders`, which CS-4.2 places in schema `shop` of the default DuckDB database); a case whose point is a source of three parts keeps it and sets `clickhouse: e6_01`. Golden ClickHouse SQL files follow CS-2.10a like every golden file. |
| CS-2.4 | `rows` are compared with NULLs equal, decimals exactly, floats within `tolerance` (relative), and the row order as declared. |
| CS-2.4a | **`rows` file.** CSV as RFC 4180, UTF-8. The first line holds the output names, compared in order with the plan's `result_schema`. An empty unquoted field is NULL and `""` is the empty text. Each value is read by its column's `logical_type`: integers and decimals as exact numbers (`45` equals `45.00`); floats within `tolerance` when the case gives one, exactly otherwise; dates as `YYYY-MM-DD`; timestamps in CON-4 form; booleans as `true` or `false`. |
| CS-2.4b | `expect.warnings` lists codes each reported by a warning of its own: the load warnings for every outcome, plus the compilation warnings (`WQRY-01`, …) for `error`, `plan` and `runtime_error`. |
| CS-2.4c | A `runtime_error` case passes when executing its plan fails with an error whose message contains the expected code, and fails when execution succeeds. |
| CS-2.5 | Runtime-policy cases expect `outcome: runtime_error` with the runtime code (`RADD-01`, …). |
| CS-2.6 | The **coverage scope** is listed in `spec/coverage-scope.txt`, one entry per line; lines starting with `#` are comments. An entry is a specification path, which puts every rule ID defined in that file in scope, or a path followed by rule IDs separated by spaces, which puts only those rules in scope. `rule-coverage` fails when a rule ID in scope is not covered (T1), or when an entry names a file or a rule ID that `spec/INDEX.md` does not define in that file, and only reports other rule IDs. The specification session adds the entries of a version at its gate (7.1a). A gate before 1.0.0 adds, rule by rule, the rules its slice implements (VS-1–VS-3); a whole-file entry is added once every rule of the file is implemented, and at the latest at the 1.0.0 gate. An entry is never removed, so later work cannot lose coverage. |
| CS-2.6a | A behavioral rule is covered by a conformance case or by a test function of the repository (a Rust `#[test]` function, or a Python function or method named `test_…`, whose name is read after that prefix) whose name starts with the rule ID in lower case with `-` and `.` written `_` (`arc_5_7_…` for ARC-5.7, `mig_3_2a_…` for MIG-3.2a), with the same item and suffix matching as a case's `rules`. A rule whose observable form the case format can express also needs a case when its stage or contract is in the slice of the version (VS-1 to VS-3); the review checks it (CS-2.1). |
| CS-2.6b | A rule with no observable behavior of its own (a definition, a statement of architecture or of a producer's obligation, a superseded rule kept so that references resolve) is listed in `spec/coverage-exempt.txt`, one rule ID per line followed by its reason, and `rule-coverage` treats it as covered. Only the specification session edits that file. An exempt rule that gains observable behavior is removed from it in the same pull request. |
| CS-2.7 | `load_options` holds load options of architecture section 3.5 (`ossie_format`, `model_name`, `namespace`, `grant_authenticated_to_all`); an option it omits takes its default. It lets a case cover a rule that applies only with a load option. |
| CS-2.9 | `location`, when present, is compared with the location of the first error: `document` and `pointer` exactly, and `line` and `column` when the case gives them (PLN-5). |
| CS-2.10 | **Golden JSON files.** `lineage`, `audit` and `document` name golden JSON files next to the case. A golden JSON file is compared byte for byte after canonical JSON: the runner parses the golden file and the produced value, serializes both as RFC 8785 canonical JSON, and the case passes only when the two byte sequences are equal. A golden file that is not valid JSON, or that has a duplicate member name, fails the case. The stored form of a golden JSON file is fixed, so that regenerating it is deterministic: UTF-8, members in RFC 8785 order, numbers and strings written as RFC 8785 writes them, two spaces of indentation per level, one member or element per line, each member name followed by `": "` (a colon and one space), `[]` and `{}` for empty values, and one line feed at the end. A golden file not in stored form fails the case, naming the file. |
| CS-2.10a | **Regenerating golden files.** A golden file (`sql`, `lineage`, `audit`, `document`) is written by the case's pull request when the case is added. Changing an existing golden file is regenerating it: the runner writes the produced output in stored form (CS-2.10 for JSON, the plan's `sql` text unchanged for `sql`), only for cases named explicitly, never in CI. CS-2.3 applies to every golden file: a regeneration pull request touches only golden files and states, for each file, which rule or contract change made its content change. |
| CS-2.10b | **Contract versions in golden files.** A golden JSON file never writes the version of a contract the compiler produces: where the produced value holds such a version (a document's `contract_version`, or the `version` of a contract entry in a capability profile, DSC-4), the golden file writes the string `@schema`, and the runner replaces it with the version of that contract's schema at the commit under test before comparing (CS-2.10). Any other value, including a version in an input document of the case, is written as it is. So a contract's version bump (A5) changes its schema and examples, never a golden file. |
| CS-2.11 | `lineage` is allowed only with `outcome: plan`. Its golden file holds the value of the plan's `lineage` member (PLN-4), a JSON array, compared as CS-2.10 states. |
| CS-2.12 | `explanation` is allowed only with `outcome: plan`. It lists the `code` value (the rule ID, PLN-1) of every entry of the plan's `explanation`, in order: the case passes only when both lists have the same length and equal items at each position. `step`, `subject`, `decision` and `reason` are not compared. An empty list asserts that the explanation is empty. |
| CS-2.13 | `audit` is allowed only with `outcome: error` and `outcome: plan`. A case with `audit` is compiled with the compile option `audit: true`, every other case with `audit: false`. The runner first checks that the record's `producer` equals the `producer` of the build under test (PLN-1), then removes the `producer` member and compares the rest with the golden file, which has no `producer` member, as CS-2.10 states. The plan or the compile error is also checked against the other expectations of the case (VS-1e). |
| CS-2.14 | `cache_key` is allowed only with `outcome: plan`. It is compared exactly with the plan's `cache_key.key` (PLN-3). |
| CS-2.15 | `outcome: describe` runs `describe` with the case's `security_context`; `outcome: integrity_checks` runs `integrity_checks` with dialect `duckdb` and needs no security context. Both outcomes require `document`, the golden output document compared as CS-2.10 states, and forbid `query`, `error`, `location`, `sql`, `rows`, `row_order`, `tolerance`, `lineage`, `explanation`, `audit` and `cache_key`. `document` is allowed with no other outcome. A model that fails to load fails the case. |
| CS-2.16 | From 0.7.0, the runner also runs `explain` with the inputs of every `outcome: plan` case, and the case fails unless the returned document equals the compiled plan member for member, except `contract`, which is `menura.explanation`, and `sql` and `parameters` (architecture section 3.5). From 0.8.0, it runs `explain` for every `outcome: error` case too, and the case fails unless `explain` returns the identical compile error (PLN-8.2). In both outcomes `explain` is called with the case's compile options and `audit: true`, and the case fails if it returns an audit record (PLN-8.3). |
| CS-2.17 | `alternatives` is allowed only with `outcome: error`. It lists the `alternatives` of the first diagnostic (PLN-5.4), in order: the case passes only when both lists are equal item by item. An empty list asserts that the diagnostic has no `alternatives` member. |
| CS-2.18 | `compile_options` holds only `actor`, a string (PLN-6.8), and is allowed only with `outcome: error` and `outcome: plan`. The runner passes it to `compile` and `explain`; every other compile option keeps its default, except `audit` (CS-2.13, CS-2.16). An `audit` golden file then holds the `actor` member. |
| CS-2.8 | A case whose outcome needs stages not yet implemented is checked in parts. From 0.2.0, the runner checks its load: `load_error` cases fully, and for the other outcomes, that the model loads and reports the expected `warnings`. The query part (`error`, `plan`, `runtime_error`) is checked from the version that implements E1–E6 (0.3.0). Until then the runner reports that part as pending, never as passed, and a version gate counts only the parts that run. From 0.3.0, a query part whose compilation returns `E_NOT_YET_SUPPORTED` stays pending, unless the case expects that code (VS-4). |

## CS-3 Apache Ossie suites

- **CS-3.1** The Apache Ossie conformance suites are vendored under `conformance/ossie/` at the tracked development commit and run unchanged.
- **CS-3.2** Updating the commit is done only by the `ossie-update` skill (OSS-4.6), in its own pull request. `conformance/ossie/expected-failures.yaml` changes only in that pull request, or in the pull request of an issue that makes listed cases pass, which removes their entries (OSS-2.4a).

## CS-4 Running

- **CS-4.1** `just conformance` runs every case of `conformance/cases/` and checks the layout of `conformance/ossie/` (the stale-list rule of OSS-2.4). `just conformance <area>` runs one area. The outcomes of the Apache Ossie suites are judged only by `just conformance ossie`, the `ossie-suites` check (OSS-2.3).
- **CS-4.2** Each case runs in a fresh in-memory DuckDB database. For each source with three parts (`catalog.schema.table`), the runner attaches an in-memory database named after the first part before running `data.sql`; two-part sources (`schema.table`) live in the default database, and `data.sql` creates their schema. This applies to the compiler's own cases and to the Apache Ossie suites (CS-4.4). The session uses non-default settings (`TimeZone` not UTC, `integer_division = true`) to prove E6-4.10. The runner's DuckDB build includes the `icu` extension, which provides time zones, so no setting or query ever downloads an extension at run time.
- **CS-4.3** The runner reports, per rule ID, the cases that cover it and their result.
- **CS-4.4** For Apache Ossie suites, the runner loads the fixture with `ossie_format: development` and `grant_authenticated_to_all: true`, builds the catalog schema by reading the created tables, wraps each `query.json` in a query envelope with `time_zone: UTC`, and compares the first error code or the rows. From 0.7.0, when Layer 3 suites block, every Layer 3 case is run this way; none is only prepared (OSS-2.4b). SQL-form cases refer to relations as `<model name>.<dataset name>` (QRY-5.1).
- **CS-4.5** Runner security context for Apache Ossie suites: `{"contract": "menura.security_context", "contract_version": "1.1.0", "subject": "conformance-runner", "roles": [], "attributes": {}}`, stored as `conformance/ossie/runner-context.json`.
- **CS-4.6** Layer 2 cases: the runner wraps the text of `query.ossie.sql` in the `sql` field of a query envelope (`time_zone: UTC`), runs `expected.sql` on the fixture to obtain the expected rows, and compares them under the case's comparison rules; an expected error is compared by its first code.
- **CS-4.7** Differential cases compare the **result rows** of the compiled plan with those of an independent oracle under CS-2.4, never the SQL text alone. Each divergence is classified in a reviewed file as a bug, a known deviation or unsupported; an unclassified divergence fails. Generated inputs record their seeds. An oracle is test tooling only (ARC-5.3) and needs an approved `deny.toml` entry.
- **CS-4.8** A differential case names its reference SQL with `oracle`, a file next to the case, run on the same fixture database without the compiler. A divergence classified in `differential/divergences.yaml` (`{case, classification: bug | known_deviation | unsupported, reason}`) leaves the case pending, never passed. An entry whose case no longer diverges fails the case, and an entry that names no differential case fails the run.

## CS-5 Producer suite

Architecture 3.3, item 5: the owner of a contract publishes a suite that its producers run in
their own CI. For the catalog contracts it is a set of documents with their expected outcome.

- **CS-5.1** Each case is a folder `conformance/producers/catalog/<case-id>/` with `document.json` (a catalog schema or state document), `schema.json` for a state case (the schema document it is checked against), and `expected.yaml`: `outcome` (`accepted`, `rejected` or `ignored`) and, unless accepted, the first `code` (CAT-9).
- **CS-5.2** The suite has at least one case per code `ECAT-01`–`ECAT-03` and `WCAT-01`–`WCAT-04`, one per CAT-8 rule that applies to the document kind, and accepted cases for a partitioned relation, a non-partitioned relation and a summary build.
- **CS-5.3** The compiler's CI runs every case through its public catalog admission and checks the outcome and the first code (CAT-9.1).
- **CS-5.4** Producers take the suite from this repository at the release tag they depend on. A document suite cannot prove the producer guarantees of CAT-7.1, which compare successive publications; each producer tests those itself.
