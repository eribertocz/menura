//! The additivity of a classified metric: the declaration E0.c validated, checked against the
//! additivity deduced from its components (E0G-4.1 to E0G-4.3).

use std::collections::BTreeSet;

use super::decomposability::deduced_additivity;
use super::diagnostic::ClassificationCode;
use super::output::{Additivity, Component};
use super::rules::Problem;
use crate::extensions::additivity::DeclaredAdditivity;

/// The additivity of a metric of `components` that declares `declaration`, or the `E0G-01`
/// problem of a declaration that relaxes the deduced additivity (E0G-4.1) or that is
/// semi-additive over more than one home dataset (E0G-4.2).
///
/// Without a declaration, the additivity is the deduced one (E0G-4.3). Declaring
/// `non_additive` always restricts it (E0G-4.1, ADD-2.2).
pub(super) fn additivity(
    components: &[Component],
    declaration: Option<&DeclaredAdditivity>,
) -> Result<Additivity, Problem> {
    let deduced = deduced_additivity(components);
    match declaration {
        None => Ok(deduced),
        Some(DeclaredAdditivity::NonAdditive) => Ok(Additivity::NonAdditive),
        Some(DeclaredAdditivity::SemiAdditive(_)) => semi_additive(components, deduced),
    }
}

fn semi_additive(components: &[Component], deduced: Additivity) -> Result<Additivity, Problem> {
    if deduced == Additivity::NonAdditive {
        return Err(conflict(
            "the metric is declared semi-additive, but an algebraic or holistic component makes it non-additive (E0G-4.1)",
        ));
    }
    let home_datasets: BTreeSet<_> = components
        .iter()
        .map(|component| &component.home_dataset)
        .collect();
    if home_datasets.len() > 1 {
        return Err(conflict(
            "the metric is declared semi-additive, but its components are on more than one home dataset (E0G-4.2)",
        ));
    }
    Ok(Additivity::SemiAdditive)
}

fn conflict(message: &str) -> Problem {
    Problem {
        code: ClassificationCode::AdditivityConflict,
        message: message.to_owned(),
    }
}
