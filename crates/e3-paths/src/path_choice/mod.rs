//! Path choice: the path from each home dataset to each needed dataset, chosen by the query
//! or the only one there is, and the join tree of each branch (E3-3.2, E3-3.3, E3-4,
//! QRY-4.3).

mod chosen;
mod error;
mod homes;
mod needed;
mod search;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use error::{ChoiceInvariant, PathChoiceCode, PathChoiceError};
pub use homes::{BranchHome, component_homes};

use std::collections::BTreeMap;

use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::BoundQuery;
use menura_representations::paths::{Branch, Branches, ElementPath, JoinStep, JoinTree};

use self::chosen::ChosenPaths;
use self::needed::{NeededDataset, needed_datasets};
use self::search::{AuthorizedEdges, PathsFound};

/// The branches of `query`, one per home of `homes` (E3-4.1), ordered by home dataset
/// (E3-4.3). Each join tree holds one instance per distinct path prefix of the paths from its
/// home dataset to every dimension and filter dataset, the `date_range` dataset included, and
/// binds each element to the instance at the end of its own path (E3-4.2, E3-4.2a).
///
/// A path is the one the query's `paths` entry for that pair chooses, or else the only path
/// in the authorized subgraph (QRY-4.3, E2-4.8). The reachability checks of rows 13 to 16 of
/// E2-5 run before this function: every needed dataset is reachable from every home.
///
/// # Errors
///
/// In the order of row 17 of E2-5: [`PathChoiceError::AmbiguousPath`] for the first element,
/// in query order, that needs a dataset two or more paths lead to without a choice (E3-3.2);
/// then [`PathChoiceError::UnusedPathEntry`] for the first `paths` entry, in query order, that
/// no branch uses (E3-3.3). An invariant violation when an entry does not resolve, a needed
/// dataset is unreachable, or `homes` is empty or repeats a home dataset.
pub fn choose_paths(
    query: &BoundQuery,
    model: &InternalModel,
    mut homes: Vec<BranchHome>,
) -> Result<Branches, PathChoiceError> {
    if homes.is_empty() {
        return Err(PathChoiceError::Invariant(ChoiceInvariant::InvalidHomes));
    }
    homes.sort_by(|left, right| left.home.cmp(&right.home));
    let edges = AuthorizedEdges::new(&model.parts().graph, query.subgraph());
    let mut resolver = PathResolver {
        edges: &edges,
        chosen: ChosenPaths::new(query.paths(), &edges)?,
        found: BTreeMap::new(),
    };
    let mut paths: Vec<Vec<ElementPath>> = homes.iter().map(|_| Vec::new()).collect();
    for needed in needed_datasets(query) {
        for (home, branch_paths) in homes.iter().zip(paths.iter_mut()) {
            branch_paths.push(ElementPath {
                element: needed.element.clone(),
                steps: resolver.path(&home.home, &needed)?,
            });
        }
    }
    if let Some(unused) = resolver.chosen.first_unused() {
        return Err(PathChoiceError::UnusedPathEntry {
            from: unused.from().clone(),
            to: unused.to().clone(),
            origin: unused.origin().clone(),
        });
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
    Branches::new(branches).ok_or(PathChoiceError::Invariant(ChoiceInvariant::InvalidHomes))
}

/// The paths of a query, computed once per pair of datasets.
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
    ) -> Result<Vec<JoinStep>, PathChoiceError> {
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
            PathsFound::Several => Err(PathChoiceError::AmbiguousPath {
                home: home.clone(),
                dataset: needed.dataset.clone(),
                origin: needed.origin.clone(),
            }),
            PathsFound::None => Err(PathChoiceError::Invariant(ChoiceInvariant::Unreachable {
                home: home.clone(),
                dataset: needed.dataset.clone(),
            })),
        }
    }
}
