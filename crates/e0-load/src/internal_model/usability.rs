//! Which elements are unusable, with every query error code (E0-4.1, E0-4.2, E0-4.2a).
//!
//! The own causes come from the substages, in substage order; E0.h then propagates them,
//! transitively, to every element that reads an unusable element: the fields and dataset
//! metrics of a dataset, the fields and metrics an expression or an extension reads, and the
//! datasets and fields a relationship names. A dataset that reaches an unusable relationship
//! leading to row policies is then unusable too (`E0H-01`), and is propagated in turn with the
//! relationship that cuts it off. A propagated cause reports no new warning.

use std::collections::{BTreeSet, HashMap, VecDeque};

use menura_model::internal_model::{DiagnosticCode, NormalizedName, RowAccessCut};

use super::ModelInput;
use super::index::{ModelIndex, ReadNames};
use super::reads::reads;
use super::row_access_routes::{DeclaredGraph, UNREACHABLE_ROW_ACCESS};
use crate::classification::ClassificationCode;
use crate::expressions::{ExpressionDiagnostic, ExpressionPlace};
use crate::extensions::ExtensionCode;
use crate::outline::ModelOutline;
use crate::pointer::JsonPointer;
use crate::relationships::RelationshipCode;
use crate::structure::{CompiledVariant, ElementExpression, ElementKind as ExpressionKind};
use crate::typing::TYPING_WARNING;

/// The query error codes of every unusable element, by JSON Pointer.
#[derive(Debug, Default)]
pub(super) struct Usability {
    codes: HashMap<JsonPointer, BTreeSet<&'static str>>,
    warnings: HashMap<JsonPointer, &'static str>,
    cuts: HashMap<JsonPointer, RowAccessCut>,
}

impl Usability {
    /// The query error codes of the element at `pointer`, ordered by code; empty when usable.
    pub(super) fn codes(&self, pointer: &JsonPointer) -> Vec<DiagnosticCode> {
        self.codes
            .get(pointer)
            .map(|codes| codes.iter().map(|code| DiagnosticCode::new(code)).collect())
            .unwrap_or_default()
    }

    /// Whether the element at `pointer` is usable.
    pub(super) fn is_usable(&self, pointer: &JsonPointer) -> bool {
        self.codes.get(pointer).is_none_or(BTreeSet::is_empty)
    }

    /// The load warning of the own cause of the element at `pointer`, if it has one.
    pub(super) fn load_warning(&self, pointer: &JsonPointer) -> Option<DiagnosticCode> {
        self.warnings
            .get(pointer)
            .map(|code| DiagnosticCode::new(code))
    }

    /// Records `code` for the element at `pointer` with no load warning of its own, because
    /// its cause already has one (E0-4.2a).
    fn record_without_warning(&mut self, pointer: &JsonPointer, code: &'static str) {
        self.codes.entry(pointer.clone()).or_default().insert(code);
    }

    /// The cause of `E0H-01` of the element at `pointer`, if it has one.
    pub(super) fn row_access_cut(&self, pointer: &JsonPointer) -> Option<RowAccessCut> {
        self.cuts.get(pointer).cloned()
    }

    /// Records `cut` for the element at `pointer` unless it already has a smaller one, by
    /// relationship name, then by dataset name; whether it changed.
    fn record_cut(&mut self, pointer: &JsonPointer, cut: RowAccessCut) -> bool {
        let smaller = self
            .cuts
            .get(pointer)
            .is_none_or(|current| cut_order(&cut) < cut_order(current));
        if smaller {
            self.cuts.insert(pointer.clone(), cut);
        }
        smaller
    }

    /// Gives every element that reads an element with a cut the smallest cut it reads.
    fn propagate_cuts(&mut self, dependencies: &[(JsonPointer, JsonPointer)]) {
        let mut readers: HashMap<&JsonPointer, Vec<&JsonPointer>> = HashMap::new();
        for (reader, read) in dependencies {
            readers.entry(read).or_default().push(reader);
        }
        let mut queue: VecDeque<JsonPointer> = self.cuts.keys().cloned().collect();
        while let Some(read) = queue.pop_front() {
            let Some(cut) = self.cuts.get(&read).cloned() else {
                continue;
            };
            for reader in readers.get(&read).into_iter().flatten() {
                if self.record_cut(reader, cut.clone()) {
                    queue.push_back((*reader).clone());
                }
            }
        }
    }

    fn record(&mut self, pointer: &JsonPointer, code: &'static str, warning: &'static str) {
        self.codes.entry(pointer.clone()).or_default().insert(code);
        self.warnings.entry(pointer.clone()).or_insert(warning);
    }
}

/// The usability of every element of `model`, with the own causes of `input` propagated.
pub(super) fn usability(input: &ModelInput<'_, '_>, model: &ModelOutline<'_>) -> Usability {
    let mut usability = own_causes(input);
    let index = ModelIndex::new(model);
    let dependencies = reads(input, model, &index);
    propagate(&mut usability, &dependencies);
    // One pass is enough: a relationship made unusable by `E0H-01` touches a dataset already
    // cut off, and every dataset that reaches it reaches the same unusable relationship.
    let cut_off =
        DeclaredGraph::new(input, model).cut_off(|relationship| usability.is_usable(relationship));
    for (dataset, cut) in cut_off {
        usability.record_without_warning(dataset, UNREACHABLE_ROW_ACCESS);
        usability.record_cut(dataset, cut);
    }
    propagate(&mut usability, &dependencies);
    usability.propagate_cuts(&dependencies);
    usability
}

/// The order of cuts: by relationship name, then by dataset name (E0-4.2a).
fn cut_order(cut: &RowAccessCut) -> (&NormalizedName, &NormalizedName) {
    (&cut.relationship, &cut.dataset)
}

/// The causes every substage recorded, in substage order (E0-4 table).
fn own_causes(input: &ModelInput<'_, '_>) -> Usability {
    let mut usability = Usability::default();
    for expression in &input.structure.expressions {
        expression_cause(&mut usability, expression);
    }
    for element in &input.references.unusable {
        usability.record(
            &element.pointer,
            element.query_error.as_str(),
            element.query_error.load_code(),
        );
    }
    for metric in &input.derived.unusable {
        let code = ExtensionCode::AggregateOutsideDerived;
        usability.record(metric, code.as_str(), ClassificationCode::WARNING);
    }
    for relationship in &input.relationships.unusable {
        usability.record(
            relationship,
            RelationshipCode::INVALID_RELATIONSHIP_QUERY_ERROR,
            RelationshipCode::INVALID_RELATIONSHIP_WARNING,
        );
    }
    for element in input
        .typing
        .unusable
        .iter()
        .chain(&input.derived_typing.unusable)
    {
        usability.record(
            &element.pointer,
            element.query_error.as_str(),
            TYPING_WARNING,
        );
    }
    for metric in &input.classification.unusable {
        usability.record(
            &metric.pointer,
            metric.query_error.as_str(),
            ClassificationCode::WARNING,
        );
    }
    usability
}

/// The cause of an expression without a single `ANSI_SQL` variant or with an unknown
/// `datatype` (E0.b), or whose compiled variant does not parse.
fn expression_cause(usability: &mut Usability, expression: &ElementExpression<'_>) {
    if let Some(code) = expression.query_error() {
        let warning = if code == ElementExpression::NO_SINGLE_ANSI_SQL_QUERY_ERROR {
            "W0B-02"
        } else {
            "W0B-03"
        };
        usability.record(&expression.pointer, code, warning);
        return;
    }
    let (CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text)) = expression.variant
    else {
        return;
    };
    let place = match expression.kind {
        ExpressionKind::Field => ExpressionPlace::Field,
        ExpressionKind::Metric | ExpressionKind::DatasetMetric => ExpressionPlace::Metric,
    };
    if let Err(code) = super::trees::parse(text, place) {
        usability.record(
            &expression.pointer,
            code.as_str(),
            ExpressionDiagnostic::WARNING,
        );
    }
}

/// Merges the codes of every element into the elements that read it, until nothing changes.
/// Each element is revisited only when its codes grow, and codes are few, so the work is
/// linear in the number of dependencies times the number of distinct codes. The result is the
/// least fixed point, whatever the order of the queue.
fn propagate(usability: &mut Usability, dependencies: &[(JsonPointer, JsonPointer)]) {
    let mut readers: HashMap<&JsonPointer, Vec<&JsonPointer>> = HashMap::new();
    for (reader, read) in dependencies {
        readers.entry(read).or_default().push(reader);
    }
    let mut queue: VecDeque<JsonPointer> = usability.codes.keys().cloned().collect();
    while let Some(read) = queue.pop_front() {
        let Some(codes) = usability.codes.get(&read).cloned() else {
            continue;
        };
        for reader in readers.get(&read).into_iter().flatten() {
            let target = usability.codes.entry((*reader).clone()).or_default();
            let before = target.len();
            target.extend(codes.iter().copied());
            if target.len() > before {
                queue.push_back((*reader).clone());
            }
        }
    }
}

/// The names a tree reads; used by the dependency computation.
pub(super) fn read_names(tree: &menura_model::expression::Expression) -> ReadNames {
    ReadNames::of(&tree.json_form())
}
