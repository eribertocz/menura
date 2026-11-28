//! The check evidence of the catalog state (CAT-7.3): whether the state holds a passing result
//! of a current check, run on the current data versions.

use std::collections::{BTreeMap, BTreeSet};

use menura_contracts::types::catalog::{
    CatalogStateDocument, CheckResult, RelationData, StateCheckKind, VerifiedVersion,
};
use menura_contracts::types::common::{CheckStatus, Token};
use menura_model::internal_model::{CheckKind, InternalModel, NormalizedName, RecordedCheck};

/// The evidence E5 may use: the model's current checks and the admitted catalog state.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Evidence<'a> {
    model: &'a InternalModel,
    state: Option<&'a CatalogStateDocument>,
}

impl<'a> Evidence<'a> {
    /// The evidence of `state` for the checks of `model`. `None` is no state, or a state the
    /// handle ignored: it holds no evidence (E5-2.1, CHK-4.1).
    pub(crate) fn new(model: &'a InternalModel, state: Option<&'a CatalogStateDocument>) -> Self {
        Self { model, state }
    }

    /// The admitted catalog state, or `None` when there is none or the handle ignored it.
    pub(crate) fn state(&self) -> Option<&'a CatalogStateDocument> {
        self.state
    }

    /// Whether the state holds evidence of the check of `kind` for `target` (CAT-7.3): a
    /// result whose `status` is `passed`, whose `definition_hash` is the current one (E0H-4.3),
    /// and whose `verified` lists exactly the current versions of the relations the check reads
    /// (its CHK-2 `sources`): the `to` relation for `key_uniqueness`, the `from` and `to`
    /// relations for `orphan_keys` and `temporal_orphans`, whose `target` is a relationship, and
    /// the relation of the dataset that declares the population for `population`, whose
    /// `target` is a population (VAL-2.14, VAL-2.15). Only those four kinds are evidence for
    /// E5; `summary` and `semi_additive` have none.
    pub(crate) fn holds(&self, kind: CheckKind, target: &NormalizedName) -> bool {
        let Some(state) = self.state else {
            return false;
        };
        let Some(state_kind) = evidence_check_kind(kind) else {
            return false;
        };
        let Some(current) = self.current_check(kind, target) else {
            return false;
        };
        let Some(sources) = self.check_sources(kind, target) else {
            return false;
        };
        state.checks.iter().flatten().any(|result| {
            is_current_result(result, state_kind, current)
                && sources
                    .iter()
                    .all(|source| verifies_current_versions(state, source, &result.verified))
        })
    }

    /// The model's current check of `kind` for `target`.
    fn current_check(&self, kind: CheckKind, target: &NormalizedName) -> Option<&'a RecordedCheck> {
        self.model
            .parts()
            .checks
            .iter()
            .find(|check| check.kind == kind && check.target == target.as_str())
    }

    /// The relations the check of `kind` for `name` reads (CHK-2 `sources`): for a
    /// relationship check, those of the `to` dataset, and also of the `from` dataset for
    /// `orphan_keys` and `temporal_orphans`; for a `population` check, that of the dataset that
    /// declares the population.
    fn check_sources(&self, kind: CheckKind, name: &NormalizedName) -> Option<BTreeSet<&'a str>> {
        if kind == CheckKind::Population {
            let declaring = self.model.parts().datasets.iter().find(|dataset| {
                dataset
                    .populations
                    .iter()
                    .any(|population| &population.name == name)
            })?;
            return Some(BTreeSet::from([declaring.source.as_str()]));
        }
        let relationship = self.model.relationship(name)?;
        let to = self.model.dataset(&relationship.to)?;
        let mut sources = BTreeSet::from([to.source.as_str()]);
        if matches!(kind, CheckKind::OrphanKeys | CheckKind::TemporalOrphans) {
            sources.insert(self.model.dataset(&relationship.from)?.source.as_str());
        }
        Some(sources)
    }
}

/// The kind of `checks` a check of `kind` is reported as, if E5 may use it as evidence.
fn evidence_check_kind(kind: CheckKind) -> Option<StateCheckKind> {
    match kind {
        CheckKind::KeyUniqueness => Some(StateCheckKind::KeyUniqueness),
        CheckKind::OrphanKeys => Some(StateCheckKind::OrphanKeys),
        CheckKind::TemporalOrphans => Some(StateCheckKind::TemporalOrphans),
        CheckKind::Population => Some(StateCheckKind::Population),
        CheckKind::Summary | CheckKind::SemiAdditive => None,
    }
}

/// Whether `result` is a passing result of the `current` check, reported as `state_kind`.
fn is_current_result(
    result: &CheckResult,
    state_kind: StateCheckKind,
    current: &RecordedCheck,
) -> bool {
    result.check == state_kind
        && result.target == current.target
        && result.status == CheckStatus::Passed
        && result.definition_hash == current.definition_hash
}

/// Whether the entries of `verified` for the relation `source` are exactly its current
/// versions in `state`: its single version when it is not partitioned, or the version of every
/// one of its periods. Tokens are compared by equality only (CAT-6.1). A relation the state
/// does not list, or a summary, has no current version to verify.
fn verifies_current_versions(
    state: &CatalogStateDocument,
    source: &str,
    verified: &[VerifiedVersion],
) -> bool {
    let Some(relation) = state
        .relations
        .iter()
        .find(|relation| relation.source == source)
    else {
        return false;
    };
    let entries: Vec<&VerifiedVersion> = verified
        .iter()
        .filter(|entry| entry.source == source)
        .collect();
    match &relation.data {
        RelationData::Version(version) => {
            matches!(entries.as_slice(), [entry] if entry.period.is_none() && &entry.version == version)
        }
        RelationData::Partitions(partitions) => {
            let current: BTreeMap<&str, &Token> = partitions
                .versions
                .iter()
                .map(|period| (period.period.as_str(), &period.version))
                .collect();
            entries.len() == current.len()
                && entries.iter().all(|entry| {
                    entry
                        .period
                        .as_deref()
                        .and_then(|period| current.get(period))
                        .is_some_and(|version| *version == &entry.version)
                })
        }
        RelationData::AggregateBuild(_) => false,
    }
}
