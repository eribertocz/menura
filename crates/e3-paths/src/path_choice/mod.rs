//! Path choice: the path from each home dataset to each needed dataset, chosen by the query
//! or the only one there is, and the join tree of each branch (E3-3.2, E3-3.3, E3-4,
//! QRY-4.3).

#[cfg(test)]
mod ambiguity_tests;
#[cfg(test)]
mod branch_tests;
mod chosen;
mod homes;
mod search;
#[cfg(test)]
mod test_support;

pub(crate) use homes::{BranchHome, component_homes};

use std::collections::BTreeMap;

use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::BoundQuery;
use menura_representations::paths::{Branch, Branches, ElementPath, JoinStep, JoinTree};

use self::chosen::ChosenPaths;
use self::search::{AuthorizedEdges, PathsFound};
use crate::error::{PathCode, PathDiagnostic, PathError};
use crate::needed::{NeededDataset, NeededDatasets};
use crate::violation::InvariantViolation;

/// The branches of `query`, one per home of `homes` (E3-4.1), ordered by home dataset
/// (E3-4.3). Each join tree holds one instance per distinct path prefix of the paths from its
/// home dataset to every dimension and filter dataset of `needed`, the `date_range` dataset
/// included, and binds each element to the instance at the end of its own path (E3-4.2,
/// E3-4.2a).
///
/// A path is the one the query's `paths` entry for that pair chooses, or else the only path
/// in the authorized subgraph (QRY-4.3, E2-4.8). The reachability checks of E2-5 rows 13 to 16
/// run before: every needed dataset is reachable from every home.
///
/// # Errors
///
/// In the order of E2-5 row 17: `E_AMBIGUOUS_PATH` for the first element, in query order,
/// that needs a dataset two or more paths lead to without a choice (E3-3.2), located at that
/// element; then `E3-01` for the first `paths` entry, in query order, that no branch uses
/// (E3-3.3), located at that entry. An [`InvariantViolation`] when an entry does not resolve,
/// a needed dataset is unreachable, or two homes share a dataset.
pub(crate) fn choose_paths(
    query: &BoundQuery,
    model: &InternalModel,
    needed: &NeededDatasets<'_>,
    mut homes: Vec<BranchHome>,
) -> Result<Branches, PathError> {
    homes.sort_by(|left, right| left.home.cmp(&right.home));
    let edges = AuthorizedEdges::new(&model.parts().graph, query.subgraph());
    let mut resolver = PathResolver {
        edges: &edges,
        chosen: ChosenPaths::new(query.paths(), &edges)?,
        found: BTreeMap::new(),
    };
    let mut paths: Vec<Vec<ElementPath>> = homes.iter().map(|_| Vec::new()).collect();
    for element in needed.dimensions.iter().chain(&needed.filters) {
        for (home, branch_paths) in homes.iter().zip(paths.iter_mut()) {
            branch_paths.push(ElementPath {
                element: element.element.clone(),
                steps: resolver.path(&home.home, element)?,
            });
        }
    }
    if let Some(unused) = resolver.chosen.first_unused() {
        let message = format!(
            "no branch uses the path from `{}` to `{}`; remove it from `paths`",
            unused.from().as_str(),
            unused.to().as_str()
        );
        return Err(
            PathDiagnostic::new(PathCode::UnusedPathEntry, message, unused.origin()).into(),
        );
    }
    let branches = homes
        .into_iter()
        .zip(paths)
        .map(|(home, paths)| Branch {
            home: home.home,
            components: home.components,
            join_tree: JoinTree::from_paths(paths),
        })
        .collect();
    Branches::new(branches).ok_or_else(|| InvariantViolation::DuplicateHomeDataset.into())
}

/// The paths of a query, searched once per pair of datasets.
struct PathResolver<'model, 'query> {
    edges: &'model AuthorizedEdges<'model>,
    chosen: ChosenPaths<'query>,
    found: BTreeMap<(NormalizedName, NormalizedName), Vec<JoinStep>>,
}

impl PathResolver<'_, '_> {
    /// The steps from `home` to the dataset `needed` reads: empty for the home dataset itself
    /// (E3-3.1), the chosen path when the query chooses one, else the only path.
    fn path(
        &mut self,
        home: &NormalizedName,
        needed: &NeededDataset<'_>,
    ) -> Result<Vec<JoinStep>, PathError> {
        if home == needed.dataset {
            return Ok(Vec::new());
        }
        if let Some(steps) = self.chosen.take(home, needed.dataset) {
            return Ok(steps);
        }
        let pair = (home.clone(), needed.dataset.clone());
        if let Some(steps) = self.found.get(&pair) {
            return Ok(steps.clone());
        }
        match self.edges.paths(home, needed.dataset) {
            PathsFound::One(steps) => {
                self.found.insert(pair, steps.clone());
                Ok(steps)
            }
            PathsFound::Several => {
                let message = format!(
                    "more than one path leads from `{}` to `{}`; choose one in `paths`",
                    home.as_str(),
                    needed.dataset.as_str()
                );
                Err(PathDiagnostic::new(PathCode::AmbiguousPath, message, needed.origin).into())
            }
            PathsFound::None => Err(InvariantViolation::UnreachableDataset {
                home: home.clone(),
                dataset: needed.dataset.clone(),
            }
            .into()),
        }
    }
}
