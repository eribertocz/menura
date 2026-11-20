//! Which elements are unusable, with every query error code (E0-4.1, E0-4.2, E0-4.2a).
//!
//! The own causes come from the substages, in substage order; E0.h then propagates them,
//! transitively, to every element that reads an unusable element: the fields and dataset
//! metrics of a dataset, the fields and metrics an expression or an extension reads, and the
//! datasets and fields a relationship names. A propagated cause reports no new warning.

use std::collections::{BTreeSet, HashMap, VecDeque};

use menura_model::internal_model::DiagnosticCode;

use super::ModelInput;
use super::index::{ModelIndex, ReadNames};
use super::reads::reads;
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
    usability
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
