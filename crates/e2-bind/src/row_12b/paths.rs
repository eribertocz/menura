//! `EQRY-12` (QRY-4.3): the relationships of each path chain from `from` to `to`, and no two
//! entries share a pair.

use std::collections::BTreeSet;

use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::BoundPath;

use crate::error::{BindCode, BindDiagnostic};

/// The first path, in query order, that does not chain or repeats the pair of an earlier one.
pub(crate) fn check_paths(
    model: &InternalModel,
    paths: &[BoundPath],
) -> Result<(), BindDiagnostic> {
    let mut pairs = BTreeSet::new();
    for path in paths {
        if !chains(model, path) {
            return Err(invalid(
                path,
                "the relationships of this path do not lead, one after another, from `from` \
                 to `to`",
            ));
        }
        if !pairs.insert((path.from(), path.to())) {
            return Err(invalid(
                path,
                "an earlier path has the same `from` and `to`; give at most one path per pair",
            ));
        }
    }
    Ok(())
}

/// Whether each relationship continues from the dataset the previous one reached, starting at
/// `from` and ending at `to`. A step may cross a relationship in either direction.
fn chains(model: &InternalModel, path: &BoundPath) -> bool {
    let mut current: &NormalizedName = path.from();
    for name in path.relationships() {
        let Some(relationship) = model.relationship(name) else {
            return false;
        };
        current = if &relationship.from == current {
            &relationship.to
        } else if &relationship.to == current {
            &relationship.from
        } else {
            return false;
        };
    }
    current == path.to()
}

fn invalid(path: &BoundPath, message: &str) -> BindDiagnostic {
    BindDiagnostic::new(BindCode::InvalidPath, message.to_owned(), path.origin())
}
