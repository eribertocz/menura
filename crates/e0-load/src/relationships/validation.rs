//! The checks of each relationship (E0E-3): which relationships are invalid, which are excluded
//! or never take part in a path, and which become edges.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::declarations::{DatasetDeclaration, Place, RelationshipDeclaration, WrittenName};
use super::diagnostic::{RelationshipCode, RelationshipDiagnostic};
use crate::names::Name;
use crate::pointer::JsonPointer;

/// What the checks look datasets up in.
pub(crate) struct Lookup<'model, 'document> {
    /// The first dataset of each name.
    by_name: HashMap<&'model Name, &'model DatasetDeclaration<'document>>,
    /// The JSON Pointers of the summary datasets.
    summaries: &'model HashSet<&'model JsonPointer>,
    /// The JSON Pointers of the datasets and relationships an earlier substage made unusable.
    unusable: &'model HashSet<&'model JsonPointer>,
}

impl<'model, 'document> Lookup<'model, 'document> {
    /// The lookup of `datasets`, where `summaries` are summary datasets and `unusable` are the
    /// datasets and relationships an earlier substage made unusable.
    pub(crate) fn new(
        datasets: &'model [DatasetDeclaration<'document>],
        summaries: &'model HashSet<&'model JsonPointer>,
        unusable: &'model HashSet<&'model JsonPointer>,
    ) -> Self {
        let mut by_name = HashMap::new();
        for dataset in datasets {
            by_name.entry(&dataset.name.name).or_insert(dataset);
        }
        Self {
            by_name,
            summaries,
            unusable,
        }
    }

    /// The first dataset that `end` names.
    fn dataset(&self, end: &WrittenName<'_>) -> Option<&'model DatasetDeclaration<'document>> {
        self.by_name.get(&end.name).copied()
    }

    /// Whether `end` names a summary dataset.
    fn is_summary(&self, end: &WrittenName<'_>) -> bool {
        self.dataset(end)
            .is_some_and(|dataset| self.summaries.contains(&dataset.place.pointer))
    }

    /// Whether an earlier substage made the element at `pointer` unusable.
    fn is_unusable(&self, pointer: &JsonPointer) -> bool {
        self.unusable.contains(pointer)
    }
}

/// What a relationship is for the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict<'model, 'document> {
    /// Invalid (`E0E-01`, `E0E-02`): unusable and excluded from the graph (E0-4.1).
    Invalid,
    /// Already unusable, by itself or by one of its datasets (E0-4.2): excluded from the graph
    /// without a diagnostic of its own here.
    Excluded,
    /// A self-relationship: allowed, never in a path (E0E-3.5).
    SelfRelationship,
    /// An edge of the graph (E0E-2.2).
    Edge {
        /// The dataset on the many side.
        from: &'model DatasetDeclaration<'document>,
        /// The dataset on the one side.
        to: &'model DatasetDeclaration<'document>,
        /// Whether `from_columns` also matches a key of `from` (E0E-3.4).
        one_to_one: bool,
    },
}

/// Checks `relationship`, adds its diagnostics to `diagnostics` and tells what it is for the
/// graph.
pub(crate) fn check<'model, 'document>(
    lookup: &Lookup<'model, 'document>,
    relationship: &RelationshipDeclaration<'_>,
    diagnostics: &mut Vec<RelationshipDiagnostic>,
) -> Verdict<'model, 'document> {
    let invalid = invalid_diagnostics(lookup, relationship);
    if !invalid.is_empty() {
        diagnostics.extend(invalid);
        return Verdict::Invalid;
    }
    let (Some(from), Some(to)) = (
        lookup.dataset(&relationship.from),
        lookup.dataset(&relationship.to),
    ) else {
        return Verdict::Excluded;
    };
    let pointers = [
        &relationship.place.pointer,
        &from.place.pointer,
        &to.place.pointer,
    ];
    if pointers
        .into_iter()
        .any(|pointer| lookup.is_unusable(pointer))
    {
        return Verdict::Excluded;
    }
    if !is_key(to, &relationship.to_columns) {
        diagnostics.push(undeclared_key(relationship));
    }
    if from.place.pointer == to.place.pointer {
        diagnostics.push(self_relationship(relationship));
        return Verdict::SelfRelationship;
    }
    Verdict::Edge {
        from,
        to,
        one_to_one: is_key(from, &relationship.from_columns),
    }
}

/// The `E0E-01` and `E0E-02` diagnostics of `relationship`.
fn invalid_diagnostics(
    lookup: &Lookup<'_, '_>,
    relationship: &RelationshipDeclaration<'_>,
) -> Vec<RelationshipDiagnostic> {
    let mut diagnostics = Vec::new();
    let (from_count, to_count) = (
        relationship.from_columns.len(),
        relationship.to_columns.len(),
    );
    if from_count != to_count {
        diagnostics.push(diagnostic(
            RelationshipCode::ColumnCountMismatch,
            &relationship.place,
            format!(
                "relationship `{}` has {from_count} `from_columns` and {to_count} `to_columns`; it is excluded from the graph",
                relationship.name.text
            ),
        ));
    }
    for end in [&relationship.from, &relationship.to] {
        if lookup.is_summary(end) {
            diagnostics.push(diagnostic(
                RelationshipCode::SummaryEnd,
                &end.place,
                format!(
                    "relationship `{}` refers to the summary dataset `{}`; it is excluded from the graph",
                    relationship.name.text, end.text
                ),
            ));
        }
    }
    diagnostics
}

/// Whether `columns` matches the primary key or a unique key of `dataset` (E0E-3.2, E0E-3.4):
/// both hold the same set of normalized names, whatever their order and repetition (E0E-3.8).
pub(crate) fn is_key(dataset: &DatasetDeclaration<'_>, columns: &[Name]) -> bool {
    let columns: BTreeSet<&Name> = columns.iter().collect();
    dataset
        .keys
        .iter()
        .any(|key| key.len() == columns.len() && columns.iter().all(|column| key.contains(*column)))
}

fn undeclared_key(relationship: &RelationshipDeclaration<'_>) -> RelationshipDiagnostic {
    diagnostic(
        RelationshipCode::UndeclaredKey,
        &relationship.to_columns_place,
        format!(
            "the `to_columns` of relationship `{}` are not a declared key of `{}`, so the data could violate its many-to-one cardinality",
            relationship.name.text, relationship.to.text
        ),
    )
}

fn self_relationship(relationship: &RelationshipDeclaration<'_>) -> RelationshipDiagnostic {
    diagnostic(
        RelationshipCode::SelfRelationship,
        &relationship.place,
        format!(
            "relationship `{}` relates `{}` to itself; it is never used in a path",
            relationship.name.text, relationship.from.text
        ),
    )
}

/// A diagnostic about the node at `place`.
pub(crate) fn diagnostic(
    code: RelationshipCode,
    place: &Place,
    message: String,
) -> RelationshipDiagnostic {
    RelationshipDiagnostic {
        code,
        pointer: place.pointer.clone(),
        location: place.location,
        message,
    }
}
