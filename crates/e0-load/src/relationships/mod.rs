//! Stage E0.e: builds the relationship graph and finds, at load time, what could later produce
//! an ambiguous join or an inflated number (E0E-1 to E0E-7).
//!
//! Every dataset except summaries is a node; every usable relationship between two different
//! datasets is an edge, from the many side to the one side. A relationship whose column lists
//! differ in length (`E0E-01`) or that touches a summary (`E0E-02`) is unusable and reported as
//! `W0E-04`; a relationship or dataset an earlier substage made unusable is left out without a
//! diagnostic here (E0-4.2). For each dataset, the datasets it reaches are computed with whether
//! one path or more lead there, and every pair with more than one path is reported as `W0E-01`.
//! Paths are not enumerated: E3 does that on demand.
//!
//! Reachability inside extensions is checked by each extension with its own codes: the fields
//! of additivity declarations by [`additivity::check_additivity_reachability`] (`E0C-16`,
//! ADD-5.1.6), and the time dimension of context metrics by
//! [`context::check_context_reachability`] (`E0C-61`, CTX-5.1.4).

pub(crate) mod additivity;
pub(crate) mod context;
mod declarations;
mod diagnostic;
mod dominators;
mod graph;
mod home_datasets;
mod output;
pub(crate) mod paths;
mod reach;
mod validation;

pub(crate) use diagnostic::{RelationshipCode, RelationshipDiagnostic};
pub(crate) use output::{PathCount, RelationshipGraph, Relationships};

use std::collections::HashSet;

use crate::extensions::{Extensions, ObjectKind};
use crate::pointer::{JsonPointer, position};
use crate::read::Node;
use crate::references::References;
use crate::structure::SelectedModel;
use declarations::DatasetDeclaration;
use output::DatasetPair;
use validation::{Lookup, Verdict, check, diagnostic};

/// The extension key that makes a dataset a summary (extension summaries).
const SUMMARY_KEY: &str = "aggregate";

/// Analyzes the relationships of the semantic model `model` selected in `document`.
///
/// `extensions` are the entries of ours that E0.c accepted, which tell the summaries apart, and
/// `references` the elements E0.d made unusable. Relationship analysis never rejects the model:
/// every diagnostic is a load warning (E0-4).
pub(crate) fn analyze_relationships(
    document: &Node,
    model: &SelectedModel<'_>,
    extensions: &Extensions,
    references: &References,
) -> Relationships {
    let datasets = declarations::datasets(model.node, &model.pointer);
    let relationships = declarations::relationships(model.node, &model.pointer);
    let summaries = summaries(extensions);
    let unusable: HashSet<&JsonPointer> = references
        .unusable
        .iter()
        .map(|element| &element.pointer)
        .collect();
    let temporal: HashSet<&JsonPointer> = extensions
        .temporal
        .iter()
        .map(|declaration| &declaration.relationship)
        .collect();
    let lookup = Lookup::new(&datasets, &summaries, &unusable, &temporal);
    let mut warnings = Vec::new();
    let judged: Vec<_> = relationships
        .iter()
        .map(|relationship| (relationship, check(&lookup, relationship, &mut warnings)))
        .collect();
    let nodes = graph::nodes(&datasets, &summaries);
    let graph = graph::build(&nodes, &judged);
    let (reachable, several_paths) = paths::reachability(&graph);
    let mut sequence = vec![0; warnings.len()];
    for (sequence_number, pair) in several_paths.iter().enumerate() {
        if let Some(warning) = several_paths_warning(&nodes, *pair) {
            warnings.push(warning);
            sequence.push(sequence_number);
        }
    }
    let warnings = sorted(document, warnings, &sequence);
    let unusable = judged
        .iter()
        .filter(|(_, verdict)| *verdict == Verdict::Invalid)
        .map(|(relationship, _)| relationship.place.pointer.clone())
        .collect();
    Relationships {
        graph,
        reachable,
        several_paths,
        unusable,
        warnings,
    }
}

/// The JSON Pointers of the summary datasets: those whose entry of ours has the summaries key.
fn summaries(extensions: &Extensions) -> HashSet<&JsonPointer> {
    extensions
        .entries
        .iter()
        .filter(|entry| entry.object == ObjectKind::Dataset)
        .filter(|entry| entry.extensions.contains_key(SUMMARY_KEY))
        .map(|entry| &entry.object_pointer)
        .collect()
}

/// The `W0E-01` diagnostic of `pair`, located at the `from` dataset of the pair (E0E-4.2).
fn several_paths_warning(
    nodes: &[&DatasetDeclaration<'_>],
    pair: DatasetPair,
) -> Option<RelationshipDiagnostic> {
    let from = nodes.get(pair.from.0)?;
    let to = nodes.get(pair.to.0)?;
    Some(diagnostic(
        RelationshipCode::SeveralPaths,
        &from.place,
        format!(
            "there is more than one path from `{}` to `{}`; a query that needs it must name the path",
            from.name.text, to.name.text
        ),
    ))
}

/// `warnings` in document order, then by code (E0-4.3), then by `sequence`, which orders the
/// `W0E-01` diagnostics with the same `from` dataset by their target in document order
/// (E0E-4.2). Graph order is document order, so the order of [`DatasetPair`]s is that order.
fn sorted(
    document: &Node,
    warnings: Vec<RelationshipDiagnostic>,
    sequence: &[usize],
) -> Vec<RelationshipDiagnostic> {
    let mut keyed: Vec<_> = warnings
        .into_iter()
        .zip(sequence.iter().copied())
        .map(|(warning, order)| {
            (
                (position(document, &warning.pointer), warning.code, order),
                warning,
            )
        })
        .collect();
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.into_iter().map(|(_, warning)| warning).collect()
}

#[cfg(test)]
mod tests;
