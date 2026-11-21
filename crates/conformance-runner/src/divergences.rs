//! The reviewed file of classified divergences of differential cases (CS-4.7):
//! `conformance/differential/divergences.yaml`.
//!
//! Every entry names a differential case whose rows differ from those of its oracle, and says
//! whether the difference is a bug, a known deviation or unsupported by the oracle. A divergence
//! without an entry fails its case.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

use crate::case::CaseId;
use crate::error::{RunnerError, RunnerErrorKind};

/// The folder of `conformance/` that holds the reviewed file.
pub(crate) const DIFFERENTIAL_FOLDER: &str = "differential";

/// The name of the reviewed file inside [`DIFFERENTIAL_FOLDER`].
pub(crate) const DIVERGENCES_FILE: &str = "divergences.yaml";

/// The kind of a classified divergence (CS-4.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Classification {
    /// The compiler returns wrong rows; an issue tracks the fix.
    Bug,
    /// The compiler and the oracle differ on purpose, as the specification says.
    KnownDeviation,
    /// The oracle cannot express the query.
    Unsupported,
}

impl Classification {
    /// The classification as written in the reviewed file.
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Bug => "bug",
            Self::KnownDeviation => "known_deviation",
            Self::Unsupported => "unsupported",
        }
    }
}

/// One entry of the reviewed file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Divergence {
    /// The differential case whose rows differ from its oracle's.
    pub(crate) case: CaseId,
    /// What the divergence is.
    pub(crate) classification: Classification,
    /// Why, with the issue that tracks it when it is a bug.
    pub(crate) reason: String,
}

/// The content of the reviewed file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct DivergencesFile {
    /// Every classified divergence.
    #[serde(default)]
    divergences: Vec<Divergence>,
}

/// The classified divergences, by case.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Divergences {
    by_case: BTreeMap<CaseId, Divergence>,
}

impl Divergences {
    /// The classified divergence of `case`, if any.
    pub(crate) fn of(&self, case: &CaseId) -> Option<&Divergence> {
        self.by_case.get(case)
    }

    /// The cases with a classified divergence, sorted.
    pub(crate) fn cases(&self) -> impl Iterator<Item = &CaseId> {
        self.by_case.keys()
    }
}

/// Reads the reviewed file of `conformance_directory`; a missing file classifies nothing.
///
/// # Errors
///
/// Returns a [`RunnerError`] when the file cannot be read, is not of the expected shape, or
/// classifies the same case twice.
pub(crate) fn read_divergences(conformance_directory: &Path) -> Result<Divergences, RunnerError> {
    let path = conformance_directory
        .join(DIFFERENTIAL_FOLDER)
        .join(DIVERGENCES_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => parse_divergences(&text).map_err(|kind| RunnerError::new(kind, &path)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Divergences::default()),
        Err(error) => Err(RunnerError::new(RunnerErrorKind::Unreadable(error), &path)),
    }
}

/// Parses the text of the reviewed file.
///
/// # Errors
///
/// Returns the [`RunnerErrorKind`] of the first problem: a shape error, or a case classified
/// twice.
pub(crate) fn parse_divergences(text: &str) -> Result<Divergences, RunnerErrorKind> {
    let file: DivergencesFile = serde_saphyr::from_str(text)
        .map_err(|error| RunnerErrorKind::Malformed(error.to_string()))?;
    let mut by_case = BTreeMap::new();
    for divergence in file.divergences {
        let case = divergence.case.clone();
        if by_case.insert(case.clone(), divergence).is_some() {
            return Err(RunnerErrorKind::RepeatedDivergence(case.to_string()));
        }
    }
    Ok(Divergences { by_case })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs_4_7_reads_every_classification() {
        let text = "divergences:\n  \
            - {case: a-001, classification: bug, reason: 'issue #1'}\n  \
            - {case: b-001, classification: known_deviation, reason: r}\n  \
            - {case: c-001, classification: unsupported, reason: r}\n";
        let divergences = parse_divergences(text).unwrap();
        let classification = |id: &str| {
            divergences
                .of(&CaseId::new(id.to_owned()))
                .unwrap()
                .classification
        };
        assert_eq!(classification("a-001"), Classification::Bug);
        assert_eq!(classification("b-001"), Classification::KnownDeviation);
        assert_eq!(classification("c-001"), Classification::Unsupported);
        assert_eq!(divergences.cases().count(), 3);
    }

    #[test]
    fn cs_4_7_an_empty_file_classifies_nothing() {
        assert_eq!(
            parse_divergences("divergences: []\n").unwrap(),
            Divergences::default()
        );
    }

    #[test]
    fn cs_4_7_rejects_an_unknown_classification() {
        let text = "divergences:\n  - {case: a-001, classification: flaky, reason: r}\n";
        assert!(matches!(
            parse_divergences(text),
            Err(RunnerErrorKind::Malformed(_))
        ));
    }

    #[test]
    fn cs_4_7_rejects_an_entry_without_reason() {
        let text = "divergences:\n  - {case: a-001, classification: bug}\n";
        assert!(matches!(
            parse_divergences(text),
            Err(RunnerErrorKind::Malformed(_))
        ));
    }

    #[test]
    fn cs_4_7_rejects_a_case_classified_twice() {
        let text = "divergences:\n  \
            - {case: a-001, classification: bug, reason: r}\n  \
            - {case: a-001, classification: unsupported, reason: r}\n";
        assert!(matches!(
            parse_divergences(text),
            Err(RunnerErrorKind::RepeatedDivergence(_))
        ));
    }

    #[test]
    fn cs_4_7_a_missing_file_classifies_nothing() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert_eq!(
            read_divergences(&directory).unwrap(),
            Divergences::default()
        );
    }
}
