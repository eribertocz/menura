//! The fixtures of one model folder (CS-1) that the runner needs to prepare a database.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::{RunnerError, RunnerErrorKind};

/// What the runner reads from `conformance/models/<model-name>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFixture {
    /// Every relation `source` of `catalog-schema.json`, in file order.
    pub sources: Vec<String>,
    /// The text of `data.sql`.
    pub data_script: String,
}

#[derive(Deserialize)]
struct CatalogSchema {
    relations: Vec<CatalogRelation>,
}

#[derive(Deserialize)]
struct CatalogRelation {
    source: String,
}

impl ModelFixture {
    /// Reads the fixtures of the model folder `folder`.
    ///
    /// # Errors
    ///
    /// Returns a [`RunnerError`] when `catalog-schema.json` or `data.sql` cannot be read, or the
    /// catalog schema has no `relations` list of objects with a `source`.
    pub fn read(folder: &Path) -> Result<Self, RunnerError> {
        let catalog_path = folder.join("catalog-schema.json");
        let catalog_text = read_text(&catalog_path)?;
        let catalog: CatalogSchema = serde_json::from_str(&catalog_text).map_err(|error| {
            RunnerError::new(RunnerErrorKind::Malformed(error.to_string()), &catalog_path)
        })?;
        let data_script = read_text(&folder.join("data.sql"))?;
        Ok(Self {
            sources: catalog
                .relations
                .into_iter()
                .map(|relation| relation.source)
                .collect(),
            data_script,
        })
    }

    /// The names of the databases to attach before `data.sql` runs (CS-4.2): the first part of
    /// every three-part source, sorted and without repetition.
    #[must_use]
    pub fn attached_databases(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .sources
            .iter()
            .filter_map(|source| database_of(source))
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }
}

/// The path of a security context file of a model folder (CS-2 `security_context`).
#[must_use]
pub fn security_context_path(folder: &Path, file_name: &str) -> PathBuf {
    folder.join("contexts").join(file_name)
}

/// The first part of a three-part source (`catalog.schema.table`), or `None` otherwise.
fn database_of(source: &str) -> Option<&str> {
    let parts: Vec<&str> = source.split('.').collect();
    match parts.as_slice() {
        [database, _, _] => Some(database),
        _ => None,
    }
}

/// Reads a UTF-8 file of the conformance directory.
///
/// # Errors
///
/// Returns [`RunnerErrorKind::Unreadable`] when the file cannot be read.
pub(crate) fn read_text(path: &Path) -> Result<String, RunnerError> {
    fs::read_to_string(path)
        .map_err(|error| RunnerError::new(RunnerErrorKind::Unreadable(error), path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(sources: &[&str]) -> ModelFixture {
        ModelFixture {
            sources: sources.iter().map(|source| (*source).to_owned()).collect(),
            data_script: String::new(),
        }
    }

    #[test]
    fn cs_4_2_attaches_the_first_part_of_three_part_sources_once() {
        let fixture = fixture(&["shop.main.orders", "shop.main.balances", "hr.main.staff"]);
        assert_eq!(fixture.attached_databases(), vec!["hr", "shop"]);
    }

    #[test]
    fn cs_4_2_attaches_nothing_for_two_part_sources() {
        let fixture = fixture(&["main.orders"]);
        assert!(fixture.attached_databases().is_empty());
    }
}
