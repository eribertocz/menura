//! Runs the producer suite of `conformance/producers/catalog/` through the public catalog
//! admission and checks its coverage (CS-5).

#![expect(
    clippy::disallowed_methods,
    reason = "A2: the test reads the conformance directory, like the runner itself"
)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use menura_contracts::types::catalog::{CatalogStateDocument, RelationData};
use menura_contracts::{AdmissionError, admit_catalog_schema, admit_catalog_state};
use serde::Deserialize;

// `cfg(test)` marks the helpers as test code, where `unwrap` and `panic!` are allowed
// (clippy.toml).

/// Every code of CAT-9 (CS-5.2).
const CODES: [&str; 7] = [
    "ECAT-01", "ECAT-02", "ECAT-03", "WCAT-01", "WCAT-02", "WCAT-03", "WCAT-04",
];

/// The CAT-8 rules each document kind is checked against at admission. CAT-8.8 needs the model
/// and CAT-8.9 is a producer guarantee (CAT-7.1.5), so neither applies to a document alone.
const SCHEMA_RULES: [&str; 3] = ["8-1", "8-2", "8-3"];
const STATE_RULES: [&str; 5] = ["8-1", "8-4", "8-5", "8-6", "8-7"];

/// `expected.yaml` of a case (CS-5.1).
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Expected {
    outcome: Outcome,
    code: Option<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum Outcome {
    Accepted,
    Rejected,
    Ignored,
}

/// The document kind of a case: a state case has `schema.json` (CS-5.1).
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Kind {
    Schema,
    State,
}

impl Kind {
    fn prefix(self) -> &'static str {
        match self {
            Self::Schema => "schema-",
            Self::State => "state-",
        }
    }

    fn failure(self) -> Outcome {
        match self {
            Self::Schema => Outcome::Rejected,
            Self::State => Outcome::Ignored,
        }
    }
}

/// One case folder, read and run.
struct Case {
    id: String,
    kind: Kind,
    expected: Expected,
    actual: Expected,
    admitted_state: Option<CatalogStateDocument>,
}

#[cfg(test)]
fn suite_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/producers/catalog")
}

#[cfg(test)]
fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[cfg(test)]
fn failure(kind: Kind, error: &AdmissionError) -> Expected {
    Expected {
        outcome: kind.failure(),
        code: Some(error.code.as_str().to_owned()),
    }
}

#[cfg(test)]
fn accepted() -> Expected {
    Expected {
        outcome: Outcome::Accepted,
        code: None,
    }
}

#[cfg(test)]
fn run_case(folder: &Path) -> Case {
    let id = folder.file_name().unwrap().to_string_lossy().into_owned();
    let expected: Expected = serde_saphyr::from_str(&read(&folder.join("expected.yaml")))
        .unwrap_or_else(|error| panic!("{id}: expected.yaml: {error}"));
    let document = read(&folder.join("document.json"));
    let schema_path = folder.join("schema.json");
    let (kind, actual, admitted_state) = if schema_path.is_file() {
        let schema = admit_catalog_schema(&read(&schema_path))
            .unwrap_or_else(|error| panic!("{id}: schema.json is not admitted: {error:?}"));
        match admit_catalog_state(&document, &schema) {
            Ok(state) => (Kind::State, accepted(), Some(state)),
            Err(error) => (Kind::State, failure(Kind::State, &error), None),
        }
    } else {
        let actual = match admit_catalog_schema(&document) {
            Ok(_) => accepted(),
            Err(error) => failure(Kind::Schema, &error),
        };
        (Kind::Schema, actual, None)
    };
    Case {
        id,
        kind,
        expected,
        actual,
        admitted_state,
    }
}

#[cfg(test)]
fn run_suite() -> Vec<Case> {
    let mut folders: Vec<PathBuf> = fs::read_dir(suite_directory())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    folders.sort();
    folders.iter().map(|folder| run_case(folder)).collect()
}

#[test]
fn cs_5_1_every_case_is_well_formed() {
    for case in run_suite() {
        assert!(
            case.id.starts_with(case.kind.prefix()),
            "{}: the folder name starts with `{}`",
            case.id,
            case.kind.prefix()
        );
        let outcome = case.expected.outcome;
        assert!(
            outcome == Outcome::Accepted || outcome == case.kind.failure(),
            "{}: a {:?} document is never {outcome:?}",
            case.id,
            case.kind
        );
        assert_eq!(
            outcome == Outcome::Accepted,
            case.expected.code.is_none(),
            "{}: `code` is given exactly when the document is not accepted",
            case.id
        );
    }
}

#[test]
fn cs_5_3_cat_9_1_every_case_has_its_outcome_and_first_code() {
    let cases = run_suite();
    assert!(!cases.is_empty());
    for case in cases {
        assert_eq!(case.actual, case.expected, "{}", case.id);
    }
}

#[test]
fn cs_5_2_every_code_has_a_case() {
    let codes: BTreeSet<String> = run_suite()
        .into_iter()
        .filter_map(|case| case.expected.code)
        .collect();
    let missing: Vec<&str> = CODES
        .into_iter()
        .filter(|code| !codes.contains(*code))
        .collect();
    assert!(missing.is_empty(), "codes without a case: {missing:?}");
}

#[test]
fn cs_5_2_every_applicable_cat_8_rule_has_a_case() {
    let cases = run_suite();
    let rules = [
        (Kind::Schema, &SCHEMA_RULES[..], "ECAT-03"),
        (Kind::State, &STATE_RULES[..], "WCAT-03"),
    ];
    for (kind, rules, code) in rules {
        for rule in rules {
            let prefix = format!("{}cat-{rule}-", kind.prefix());
            let covering: Vec<&Case> = cases
                .iter()
                .filter(|case| case.id.starts_with(&prefix))
                .collect();
            assert!(!covering.is_empty(), "no case named `{prefix}…`");
            for case in covering {
                assert_eq!(case.expected.code.as_deref(), Some(code), "{}", case.id);
            }
        }
    }
}

#[test]
fn cs_5_2_accepted_cases_cover_each_relation_kind() {
    let mut kinds = BTreeSet::new();
    for case in run_suite() {
        // A case proves a kind only when it is the only relation of the accepted state.
        let Some([relation]) = case
            .admitted_state
            .as_ref()
            .map(|state| &state.relations[..])
        else {
            continue;
        };
        kinds.insert(match relation.data {
            RelationData::Version(_) => "non-partitioned relation",
            RelationData::Partitions(_) => "partitioned relation",
            RelationData::AggregateBuild(_) => "summary build",
        });
    }
    let expected = BTreeSet::from([
        "non-partitioned relation",
        "partitioned relation",
        "summary build",
    ]);
    assert_eq!(kinds, expected);
}
