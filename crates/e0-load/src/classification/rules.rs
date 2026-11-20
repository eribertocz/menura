//! Splits a metric expression into its components and checks the rules of E0G-2 that E0.g
//! reports: E0G-2.1, E0G-2.3 to E0G-2.5 and E0G-2.7 (E0G-2.8).

use std::collections::BTreeSet;

use menura_model::expression::Reference;

use super::decomposability::class;
use super::diagnostic::ClassificationCode;
use super::output::Component;
use super::parts::{AggregateParts, ExpressionParts};
use crate::names::Name;

/// Where a metric is declared, which decides the dataset each of its references reads.
#[derive(Debug, Clone, Copy)]
pub(super) enum MetricScope<'model> {
    /// A model metric: a reference `dataset.field` reads the dataset it names.
    Model,
    /// A dataset metric of the development format: every reference reads this dataset
    /// (E0G-1.4).
    Dataset(&'model Name),
}

/// A rule of E0G-2 that a metric breaks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Problem {
    /// The query error code of the rule.
    pub(super) code: ClassificationCode,
    /// What the problem is, in English.
    pub(super) message: String,
}

/// The components of a metric of `parts` declared in `scope` (E0G-1.1, E0G-1.4), or, when it
/// breaks rules of E0G-2, the first of them in the precedence of E2-5 (E0G-2.8); between
/// problems with the same code, the first written.
pub(super) fn components(
    parts: &ExpressionParts<'_>,
    scope: MetricScope<'_>,
) -> Result<Vec<Component>, Problem> {
    let mut problems = Vec::new();
    if let Some(reference) = parts.unaggregated_references.first() {
        problems.push(Problem {
            code: ClassificationCode::UnaggregatedFinerGrainReference,
            message: format!(
                "the reference `{}` is outside every aggregate (E0G-2.1)",
                describe(reference)
            ),
        });
    }
    let mut components = Vec::new();
    for aggregate in &parts.aggregates {
        match home_dataset(aggregate, scope) {
            Ok(home_dataset) => components.push(Component {
                call: aggregate.call.clone(),
                filter: aggregate.filter.cloned(),
                home_dataset,
                class: class(aggregate.call),
            }),
            Err(problem) => problems.push(problem),
        }
    }
    if parts.aggregates.is_empty() {
        problems.push(Problem {
            code: ClassificationCode::NoAggregate,
            message: "the base metric has no aggregate (E0G-2.7)".to_owned(),
        });
    }
    match problems.into_iter().min_by_key(|problem| problem.code) {
        Some(problem) => Err(problem),
        None => Ok(components),
    }
}

/// The home dataset of `aggregate` (E0G-1.1, E0G-1.4), or the rule of E0G-2.3 to E0G-2.5 it
/// breaks.
fn home_dataset(aggregate: &AggregateParts<'_>, scope: MetricScope<'_>) -> Result<Name, Problem> {
    match scope {
        MetricScope::Dataset(dataset) => Ok(dataset.clone()),
        MetricScope::Model => model_home_dataset(aggregate),
    }
}

/// The home dataset of an aggregate of a model metric: the single dataset its arguments read,
/// which its `FILTER` may read too.
fn model_home_dataset(aggregate: &AggregateParts<'_>) -> Result<Name, Problem> {
    let arguments = datasets(&aggregate.argument_references);
    let mut read = arguments.iter();
    let (Some(home), None) = (read.next(), read.next()) else {
        let message = if arguments.is_empty() {
            "the arguments of an aggregate of a model-level metric read no dataset (E0G-2.5)"
        } else {
            "the arguments of an aggregate read more than one dataset (E0G-2.3)"
        };
        return Err(ambiguous(message));
    };
    if datasets(&aggregate.filter_references)
        .iter()
        .any(|dataset| dataset != home)
    {
        return Err(ambiguous(
            "the FILTER of an aggregate reads a dataset its arguments do not read (E0G-2.4)",
        ));
    }
    Ok(home.clone())
}

fn ambiguous(message: &str) -> Problem {
    Problem {
        code: ClassificationCode::AmbiguousMeasureGrain,
        message: message.to_owned(),
    }
}

/// The datasets that the references `dataset.field` of a model metric read. E0.d and E0.f
/// leave no unqualified reference in a usable model metric.
fn datasets(references: &[&Reference]) -> BTreeSet<Name> {
    references
        .iter()
        .filter_map(|reference| reference.qualifier.as_ref())
        .map(Name::of)
        .collect()
}

/// A reference as written, for messages.
fn describe(reference: &Reference) -> String {
    match &reference.qualifier {
        Some(qualifier) => format!("{}.{}", qualifier.text(), reference.name.text()),
        None => reference.name.text().to_owned(),
    }
}
