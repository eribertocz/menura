# Producer suite for the catalog contracts

This folder is the producer suite of `spec/08-conformance-suite.md` CS-5: catalog schema and
catalog state documents (`spec/contracts/catalog.md`), each with the outcome the compiler gives
it when it admits the document. A producer runs these cases in its own CI to check that it reads
the contracts the same way the compiler does.

## Which version to use

Take the suite from this repository at the release tag your producer depends on (CS-5.4). The
cases of a tag match the contract versions and the admission of that release; cases on `main`
may describe behavior that is not released yet.

## Layout

Each case is a folder `<case-id>/` (CS-5.1) with:

| File | Content |
|---|---|
| `document.json` | The document under test: a catalog schema document, or a catalog state document. It may be deliberately invalid, even not JSON at all. |
| `schema.json` | State cases only: the catalog schema document the state is checked against. It is always admitted. |
| `expected.yaml` | `outcome` and, unless the outcome is `accepted`, the first `code` (CAT-9). |

A case with `schema.json` is a state case; a case without it is a schema case.

`outcome` is one of:

- `accepted`: the document is admitted.
- `rejected`: a schema document fails; `load` fails with `code` (`ECAT-01` to `ECAT-03`).
- `ignored`: a state document fails; the state is ignored with the warning `code` (`WCAT-01` to
  `WCAT-04`), and compilation continues without it.

Only the outcome and the first code are part of a case. The order of checks (CAT-9.1) decides
which code is first: the document is JSON, then `contract` and `contract_version`, then the JSON
Schema, then, for a state, `schema_document_id`, then the CAT-8 rules. Cases whose name ends in
`-before-<check>` hold two failures and prove that order.

## Case names

The folder name says what the case is about. It starts with the document kind, `schema-` or
`state-`, followed by one of:

- `accepted-…`: a document that is admitted. The state cases cover a partitioned relation, a
  non-partitioned relation and a summary build, each alone in its state, and all three together.
- `ecat-NN-…` or `wcat-NN-…`: a failure of a contract check or of the JSON Schema.
- `cat-8-N-…`: a violation of rule CAT-8.N, so the code is `ECAT-03` for a schema and `WCAT-03`
  for a state.

## Running the cases

For each case, give `document.json` (and, for a state case, `schema.json`) to the validation
your producer runs, and compare the result with `expected.yaml`:

- for `accepted`, your validation finds no failure;
- otherwise, the first failure it reports has `code`.

The compiler runs the same cases through its public catalog admission in its own CI
(CS-5.3), in `crates/conformance-runner/tests/producer_suite.rs`, which also checks that every
code and every applicable CAT-8 rule has a case (CS-5.2).

## What the suite does not cover

- The producer guarantees of CAT-7.1, which compare successive publications. A single document
  cannot prove them; each producer tests them itself (CS-5.4). This includes CAT-8.9, which rests
  on the guarantee CAT-7.1.5.
- CAT-8.8, which compares a state with the summaries of a model, not with the documents alone.
