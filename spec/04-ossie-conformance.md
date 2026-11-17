# 4. Apache Ossie conformance

## 4.1 Normative base

| Part | Normative source | Pinned at |
|---|---|---|
| Semantic model | Apache Ossie 0.1.1 | Tag `osi-0.1.1-rc1` of the Apache Ossie repository (`core-spec/osi-schema.json`, `core-spec/spec.md`) |
| Query interfaces (Layer 2 relational, Layer 3 wide-table), expression language, conformance suites | Apache Ossie development line | The tracked commit declared in every build (`producer.ossie_tracked_commit`); today `aa500d3` |

- **OSS-1.1** The model is read normatively at 0.1.1. The query interfaces are normative at the tracked commit, because no official release contains them yet. When an official release does, it replaces the tracked commit for those parts.
- **OSS-1.2** A model document in the development-line format is accepted only with the explicit load option `ossie_format: development` (section 4.5). It is never accepted by default.

## 4.2 Conformance

- **OSS-2.1** Loading never rejects a document that the official 0.1.1 schema and reference validator both accept, except for the deviations of section 4.6.
- **OSS-2.2** An element the compiler cannot compile (an expression outside the accepted subset, an expression without an `ANSI_SQL` variant, an invalid metric) does not reject the model. It is marked **unusable** with a warning at load time, and a query that names it fails with the error Apache Ossie specifies, or the compiler's own code where Apache Ossie specifies none.
- **OSS-2.3** Where an official release ships conformance suites, passing all of them is required. Suites at the tracked commit run in CI; Layer 3 suites block the build from 0.7.0 on; Layer 2 suites run in study mode and do not block until the Layer 2 coverage they test is implemented (QRY-5).
- **OSS-2.4** For the suites that block (OSS-2.3), an expected-failure list (`conformance/ossie/expected-failures.yaml`) names every suite case not yet passing. Each entry has `reason`, `fixed_in` (the roadmap version expected to fix it) and, once one exists, `issue` (the issue that will fix it). A stale list always fails the build, whether the suite blocks or not: an entry that names no vendored case, or a vendored case that does not pass and is not listed. For a suite that blocks, a case that passes while listed also fails the build. Layer 2 suites are reported against the same list but never fail the build until they block.
- **OSS-2.5** The compiler never claims full conformance to an interface while any of its suite cases is in the expected-failure list.
- **OSS-2.6** Behavior that depends on an extension is non-portable and is never presented as Apache Ossie behavior, in documentation, explanations or diagnostics. Apache Ossie Layer 3 section 9 allows answering a query the specification rejects only when the query or model requests it explicitly; every extension of this compiler is such an explicit request.

## 4.3 Extensions are temporary

Every capability outside Apache Ossie is an extension under the `semantic_compiler` namespace.

| Step | Trigger | Action |
|---|---|---|
| Active | No equivalent in the normative release | Supported |
| Deprecated | A new official release covers the capability | The native form is adopted; the extension emits a warning with a removal date |
| Migrated | Same release | `semantic-compiler migrate` rewrites models to the native form |
| Removed | Next major version of the model contract | The extension is rejected with an error pointing to the native form |

- **OSS-3.1** Extensions SHOULD use the form Apache Ossie is most likely to adopt, so that migration is mechanical.

## 4.4 Release tracking

| ID | Requirement |
|---|---|
| OSS-4.1 | A scheduled CI job checks Apache Ossie releases and specification changes daily and opens an `ossie-watch` issue for each. A release is any tag `osi-X.Y.Z` or `osi-X.Y.Z-rcN`; a final tag ranks above its release candidates. A release issue is labeled `ossie-watch` and `spec` (OSS-4.5 updates the mapping first); a development-change issue is labeled `ossie-watch` and `impl` (OSS-4.4). |
| OSS-4.2 | A new official release opens a top-priority issue with the specification diff and the new suites already run against the compiler. |
| OSS-4.3 | Every new official release MUST be supported within **14 days**. |
| OSS-4.4 | A development-line change updates the tracked commit in its own pull request, with the suite results and the updated expected-failure list. |
| OSS-4.5 | `spec/ossie-mapping.md` lists each Apache Ossie rule with the specification rule and conformance cases that implement it, and is updated first on every change. |
| OSS-4.6 | The procedure is the `ossie-update` skill (`.claude/skills/ossie-update`). |

Watching is a project duty, not a runtime behavior: compilation stays a pure function (P2).

## 4.5 Development-line model reader

With `ossie_format: development`, E0.a and E0.b read the development-line format and map it to
the same internal model:

| Development line | Internal model |
|---|---|
| Flat root: `version`, `name`, `datasets`, `relationships`, `metrics` | One semantic model |
| `expression: {sql: …}` | The expression, interpreted as `OSSIE_SQL_2026` |
| `expression: {dialects: […]}` | As in 0.1.1 |
| `datatype` on fields and metrics | Checked against the catalog type; a mismatch is `E0F-03` |
| Dataset-scoped `metrics` | Dataset metrics: one component each, referenced as `dataset.metric`. In their expressions a bare name is a field of that dataset, `source.column` is a source column, and `COUNT(*)` counts the dataset's rows |
| `version: 0.3.0.dev0` (or the tracked commit's version) | Accepted instead of E0B-3.1 |
| `datatype` values | Mapped to logical kinds by the table of OSS-5.2 |
| `nonportable_extensions` with object `data` | Extensions; ours use the same keys as in MDL-2 |

- **OSS-5.1** The development-line JSON Schema at the tracked commit is the structure of reference for this format.
- **OSS-5.2** `datatype` mapping: `String` → `text`; `Integer` → `integer` (any width); `Decimal` → `decimal` (any precision and scale); `Float` → `float`; `Boolean` → `boolean`; `Date` → `date`; `DateTime` → `timestamp` without time zone; `DateTimeTz` → `timestamp` with time zone; `Time` → `time`. Any other value makes the element unusable (`W0B-03`).

## 4.6 Declared deviations

Each deviation is deliberate, justified, and reported with its own code.

| ID | Apache Ossie behavior | Compiler behavior | Reason |
|---|---|---|---|
| DEV-1 | The 0.1.1 reference validator accepts duplicate YAML keys (the last value wins) | Duplicate keys are an error (`E0A-04`) | A document with two readings violates P1 |
| DEV-2 | 0.1.1 compares names exactly | Names that differ only in case collide (`E0D-01`), as in the development line | Avoids models that break when the development rule becomes normative |
| DEV-3 | 0.1.1 needs no catalog | Every dataset `source` MUST be described by the catalog schema (`E0D-02`) | Types and nullability must be known to guarantee results (P1) |
| DEV-4 | 0.1.1 has no security | Datasets without `access` are inaccessible (P9), unless the load option `grant_authenticated_to_all: true` is set; that option is recorded in the explanation and the audit record of every plan | Deny by default |
| DEV-5 | The development line allows `AGG()` only of a dataset-scoped metric | Derived and context metrics may use `AGG(metric)` of a model metric | 0.1.1 has only model metrics; the form migrates mechanically |
| DEV-6 | Layer 2 applies the engine's division semantics | Division by zero yields NULL (E0F-6) | The same result on every engine (P3) |
| DEV-7 | A model may contain several semantic models, or none (`semantic_model: []`) | `load` compiles exactly one: a document without any is rejected, and with more than one the load option `model_name` is required (`E0B-07`) | One handle, one model |
| DEV-8 | Layer 2 exposes only dataset-scoped metrics as measures | A 0.1.1 model metric whose every component has home dataset `D` is exposed as a measure of relation `D` in the SQL form (QRY-5.2) | 0.1.1 has no dataset-scoped metrics |
| DEV-9 | 0.1.1 sets no identifier length | Identifiers longer than 128 characters are rejected (`E0D-01`), as in the development line | Portability across engines |
| DEV-10 | `REGEXP_LIKE` and the `MILLISECOND` part of `EXTRACT` and `DATE_PART` are REQUIRED in `OSSIE_SQL_2026` | Outside the accepted subset (`E0F-02`, E0F-2.6) | Apache Ossie fixes neither the regular expression syntax nor whether milliseconds include the seconds, so engines return different results (P1) |
