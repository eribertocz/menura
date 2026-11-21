//! The `paths` entries of a query, resolved to join steps, with the ones a branch uses
//! (QRY-4.3, E3-3.3).

use menura_model::internal_model::NormalizedName;
use menura_representations::bound_query::BoundPath;
use menura_representations::paths::JoinStep;

use super::error::{ChoiceInvariant, PathChoiceError};
use super::search::AuthorizedEdges;

/// One `paths` entry, with its steps and whether a branch used it.
#[derive(Debug)]
struct ChosenEntry<'query> {
    path: &'query BoundPath,
    steps: Vec<JoinStep>,
    used: bool,
}

/// The `paths` entries of a query, in query order.
#[derive(Debug)]
pub(crate) struct ChosenPaths<'query> {
    entries: Vec<ChosenEntry<'query>>,
}

impl<'query> ChosenPaths<'query> {
    /// The entries `paths`, each resolved to its steps over `edges`.
    ///
    /// # Errors
    ///
    /// An invariant violation when an entry names a relationship outside the authorized
    /// subgraph or does not chain from its `from` to its `to`; E2 reports both first
    /// (`E_NAME_NOT_FOUND`, `EQRY-12`).
    pub(crate) fn new(
        paths: &'query [BoundPath],
        edges: &AuthorizedEdges<'_>,
    ) -> Result<Self, PathChoiceError> {
        let entries = paths
            .iter()
            .map(|path| {
                Ok(ChosenEntry {
                    path,
                    steps: steps_of(path, edges)?,
                    used: false,
                })
            })
            .collect::<Result<_, PathChoiceError>>()?;
        Ok(Self { entries })
    }

    /// The steps the query chooses from `from` to `to`, when it chooses some; the entry is
    /// then used.
    pub(crate) fn take(
        &mut self,
        from: &NormalizedName,
        to: &NormalizedName,
    ) -> Option<Vec<JoinStep>> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.path.from() == from && entry.path.to() == to)?;
        entry.used = true;
        Some(entry.steps.clone())
    }

    /// The first entry, in query order, that no branch used.
    pub(crate) fn first_unused(&self) -> Option<&'query BoundPath> {
        self.entries
            .iter()
            .find(|entry| !entry.used)
            .map(|entry| entry.path)
    }
}

/// The steps of `path`, in order.
fn steps_of(
    path: &BoundPath,
    edges: &AuthorizedEdges<'_>,
) -> Result<Vec<JoinStep>, PathChoiceError> {
    let invariant = |violation| PathChoiceError::Invariant(violation);
    let origin = || path.origin().clone();
    let mut current = path.from();
    let mut steps = Vec::new();
    for relationship in path.relationships() {
        let edge = edges
            .edge(relationship)
            .ok_or_else(|| invariant(ChoiceInvariant::UnknownRelationship { origin: origin() }))?;
        if &edge.from != current {
            return Err(invariant(ChoiceInvariant::BrokenChain { origin: origin() }));
        }
        steps.push(JoinStep {
            relationship: edge.relationship.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
        });
        current = &edge.to;
    }
    if current == path.to() {
        Ok(steps)
    } else {
        Err(invariant(ChoiceInvariant::BrokenChain { origin: origin() }))
    }
}
