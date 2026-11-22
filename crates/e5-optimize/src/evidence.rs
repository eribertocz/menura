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

    /// Whether the state holds evidence of the check of `kind` for the relationship `target`
    /// (CAT-7.3): a result whose `status` is `passed`, whose `definition_hash` is the current
    /// one (E0H-4.3), and whose `verified` lists exactly the current versions of the relations
    /// of the relationship's `from` and `to` datasets. Only `key_uniqueness` and `orphan_keys`
    /// are evidence for E5; every other kind has none.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "read by the rewrites of E5-3.2 and E5-3.3, filled in by their own issues"
        )
    )]
    pub(crate) fn holds(&self, kind: CheckKind, target: &NormalizedName) -> bool {
        let Some(state) = self.state else {
            return false;
        };
        let Some(state_kind) = relationship_check_kind(kind) else {
            return false;
        };
        let Some(current) = self.current_check(kind, target) else {
            return false;
        };
        let Some(sources) = self.relationship_sources(target) else {
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

    /// The relations of the `from` and `to` datasets of the relationship `name`.
    fn relationship_sources(&self, name: &NormalizedName) -> Option<BTreeSet<&'a str>> {
        let relationship = self.model.relationship(name)?;
        let from = self.model.dataset(&relationship.from)?;
        let to = self.model.dataset(&relationship.to)?;
        Some(BTreeSet::from([from.source.as_str(), to.source.as_str()]))
    }
}

/// The kind of `checks` a relationship check of `kind` is reported as, if it is one.
fn relationship_check_kind(kind: CheckKind) -> Option<StateCheckKind> {
    match kind {
        CheckKind::KeyUniqueness => Some(StateCheckKind::KeyUniqueness),
        CheckKind::OrphanKeys => Some(StateCheckKind::OrphanKeys),
        CheckKind::Summary | CheckKind::Population | CheckKind::SemiAdditive => None,
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
