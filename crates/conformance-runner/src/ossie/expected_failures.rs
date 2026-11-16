//! The expected-failure list `conformance/ossie/expected-failures.yaml` (OSS-2.4).

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use crate::error::RunnerErrorKind;

/// Why a suite case does not pass yet, and when it is expected to.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedFailure {
    /// Why the case does not pass, such as `not implemented`.
    pub reason: String,
    /// The version expected to make the case pass.
    pub fixed_in: String,
}

/// The expected-failure list, keyed by case (`<suite folder>/<case folder>`).
pub type ExpectedFailures = BTreeMap<String, ExpectedFailure>;

/// Parses the text of `expected-failures.yaml`.
///
/// # Errors
///
/// Returns [`RunnerErrorKind::Malformed`] when the text is not a mapping from case keys to
/// entries with exactly `reason` and `fixed_in`.
pub fn parse_expected_failures(text: &str) -> Result<ExpectedFailures, RunnerErrorKind> {
    serde_saphyr::from_str(text).map_err(|error| RunnerErrorKind::Malformed(error.to_string()))
}

/// Checks that every entry of the list names a vendored case (OSS-2.4), so the list cannot keep
/// cases that no longer exist.
///
/// # Errors
///
/// Returns [`RunnerErrorKind::UnknownExpectedFailure`] for the first entry, in key order, that
/// names no case of `case_keys`.
pub fn check_entries(
    failures: &ExpectedFailures,
    case_keys: &BTreeSet<String>,
) -> Result<(), RunnerErrorKind> {
    match failures.keys().find(|key| !case_keys.contains(*key)) {
        Some(key) => Err(RunnerErrorKind::UnknownExpectedFailure(key.clone())),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &str = "wide_table/a01_shared_dimension:\n  reason: not implemented\n  \
                        fixed_in: \"0.7.0\"\n";

    #[test]
    fn oss_2_4_reads_the_reason_and_the_version_of_each_entry() {
        let failures = parse_expected_failures(LIST).unwrap();
        let entry = &failures["wide_table/a01_shared_dimension"];
        assert_eq!(entry.reason, "not implemented");
        assert_eq!(entry.fixed_in, "0.7.0");
    }

    #[test]
    fn oss_2_4_rejects_an_entry_without_a_version() {
        let text = "wide_table/a01_shared_dimension:\n  reason: not implemented\n";
        assert!(matches!(
            parse_expected_failures(text),
            Err(RunnerErrorKind::Malformed(_))
        ));
    }

    #[test]
    fn oss_2_4_rejects_an_entry_that_names_no_vendored_case() {
        let failures = parse_expected_failures(LIST).unwrap();
        let cases = BTreeSet::from(["wide_table/a02_count_empty_input".to_owned()]);
        assert!(matches!(
            check_entries(&failures, &cases),
            Err(RunnerErrorKind::UnknownExpectedFailure(key)) if key == "wide_table/a01_shared_dimension"
        ));
    }

    #[test]
    fn oss_2_4_accepts_entries_that_name_vendored_cases() {
        let failures = parse_expected_failures(LIST).unwrap();
        let cases = BTreeSet::from(["wide_table/a01_shared_dimension".to_owned()]);
        assert!(check_entries(&failures, &cases).is_ok());
    }
}
