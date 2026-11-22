//! SQL form: an expression with every reference written as the model element it resolved to,
//! so that later stages read it without the `FROM` relation's alias (E2-2).

use menura_model::expression::{Expression, Identifier, Reference};
use menura_model::internal_model::NormalizedName;
use menura_representations::bound_query::MetricPath;

use crate::authorized_query::ResolvedName;
use crate::walk::for_each_reference_mut;

/// `expression` with its references, in written order, replaced by `targets`, which row 3
/// resolved them to in that order: a field as `"DATASET"."FIELD"`, a dataset metric as
/// `"DATASET"."METRIC"`, and a metric of the semantic model as `"METRIC"`, each with its
/// normalized name. Returns `None` when `targets` does not hold one target per reference.
pub(crate) fn canonical(expression: &Expression, targets: &[ResolvedName]) -> Option<Expression> {
    let mut rewritten = expression.clone();
    let mut targets = targets.iter();
    let mut complete = true;
    for_each_reference_mut(&mut rewritten, &mut |reference| match targets
        .next()
        .and_then(target_reference)
    {
        Some(written) => *reference = written,
        None => complete = false,
    });
    (complete && targets.next().is_none()).then_some(rewritten)
}

/// The reference that names `target`.
fn target_reference(target: &ResolvedName) -> Option<Reference> {
    let (qualifier, name) = match target {
        ResolvedName::Field(field) => (Some(&field.dataset), &field.field),
        ResolvedName::Metric(MetricPath::Dataset { dataset, metric }) => (Some(dataset), metric),
        ResolvedName::Metric(MetricPath::SemanticModel(metric)) => (None, metric),
    };
    Some(Reference {
        qualifier: match qualifier {
            Some(dataset) => Some(delimited(dataset)?),
            None => None,
        },
        name: delimited(name)?,
    })
}

/// `name` as a delimited identifier, which E0D-1.4 reads back as `name` exactly.
fn delimited(name: &NormalizedName) -> Option<Identifier> {
    Identifier::delimited(name.as_str())
}
