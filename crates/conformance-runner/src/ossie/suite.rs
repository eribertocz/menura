//! The two vendored Apache Ossie suites and the discovery of their cases (CS-3.1).

use std::path::{Path, PathBuf};

use crate::case::RuleId;
use crate::error::{RunnerError, RunnerErrorKind};
use crate::runner::list_directory;

/// A vendored Apache Ossie suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Suite {
    /// `compliance/wide_table`: the Layer 3 wide-table query interface (CS-4.4).
    WideTable,
    /// `compliance/query`: the Layer 2 relational query interface (CS-4.6).
    Query,
}

impl Suite {
    /// Every suite, in the order the runner reports them.
    pub const ALL: [Self; 2] = [Self::WideTable, Self::Query];

    /// The folder of the suite under `conformance/ossie/`, as named upstream.
    #[must_use]
    pub fn folder_name(self) -> &'static str {
        match self {
            Self::WideTable => "wide_table",
            Self::Query => "query",
        }
    }

    /// The rule IDs a case of this suite is reported under (CS-4.3): the rule that says how the
    /// runner runs it, and OSS-2.3, which requires the suites to pass.
    #[must_use]
    pub fn rules(self) -> Vec<RuleId> {
        let running_rule = match self {
            Self::WideTable => "CS-4.4",
            Self::Query => "CS-4.6",
        };
        vec![RuleId::new(running_rule), RuleId::new("OSS-2.3")]
    }
}

/// One case of a vendored suite: a folder under `<suite>/cases/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OssieCase {
    /// The suite the case belongs to.
    pub suite: Suite,
    /// The name of the case folder, such as `a01_shared_dimension`.
    pub name: String,
    /// The folder of the suite, `conformance/ossie/<suite>/`.
    pub suite_directory: PathBuf,
}

impl OssieCase {
    /// The key of the case in `expected-failures.yaml` and in the report:
    /// `<suite folder>/<case folder>`.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}/{}", self.suite.folder_name(), self.name)
    }

    /// The folder holding the case files.
    #[must_use]
    pub fn folder(&self) -> PathBuf {
        self.suite_directory.join("cases").join(&self.name)
    }
}

/// Every case of every suite under `ossie_directory`, suites in [`Suite::ALL`] order and cases
/// sorted by folder name.
///
/// # Errors
///
/// Returns a [`RunnerError`] when a `cases` folder cannot be read or a case folder name is not
/// UTF-8.
pub fn discover_cases(ossie_directory: &Path) -> Result<Vec<OssieCase>, RunnerError> {
    let mut cases = Vec::new();
    for suite in Suite::ALL {
        let suite_directory = ossie_directory.join(suite.folder_name());
        for folder in list_directory(&suite_directory.join("cases"))? {
            if !folder.is_dir() {
                continue;
            }
            let name = folder_name(&folder)?;
            cases.push(OssieCase {
                suite,
                name,
                suite_directory: suite_directory.clone(),
            });
        }
    }
    Ok(cases)
}

fn folder_name(folder: &Path) -> Result<String, RunnerError> {
    folder
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            RunnerError::new(
                RunnerErrorKind::Malformed("the case folder name is not UTF-8".to_owned()),
                folder,
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oss_2_4b_only_the_layer_3_suite_blocks() {
        assert!(Suite::WideTable.blocks());
        assert!(!Suite::Query.blocks());
    }

    #[test]
    fn cs_4_3_case_keys_name_the_suite_and_the_folder() {
        let case = OssieCase {
            suite: Suite::WideTable,
            name: "a01_shared_dimension".to_owned(),
            suite_directory: PathBuf::from("ossie/wide_table"),
        };
        assert_eq!(case.key(), "wide_table/a01_shared_dimension");
        assert_eq!(
            case.folder(),
            Path::new("ossie/wide_table/cases/a01_shared_dimension")
        );
    }
}
