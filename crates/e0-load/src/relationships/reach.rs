//! What the reachability checks of extensions read (E0C-16, E0C-61): the datasets and
//! metrics of the model, the elements made unusable before E0.e, and the datasets a path leads
//! to from a dataset (E0E-2.3, E0E-2.4).

use std::collections::{HashMap, HashSet};

use semantic_compiler_model::expression::Expression;

use super::output::{DatasetIndex, Relationships};
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::names::Name;
use crate::outline::{DatasetOutline, ModelOutline};
use crate::pointer::JsonPointer;
use crate::references::References;
use crate::structure::{CompiledVariant, ElementExpression, ElementKind, Structure};

/// What the reachability checks read.
pub(super) struct Model<'model, 'document> {
    /// The datasets of the model, by name.
    pub(super) datasets: HashMap<&'model Name, &'model DatasetOutline<'document>>,
    /// The model metrics usable after E0.b, by JSON Pointer, with their compiled tree.
    pub(super) metrics: HashMap<&'model JsonPointer, Expression>,
    /// The dataset metrics usable after E0.b, by JSON Pointer, with the name of their own
    /// dataset (E0G-1.4).
    pub(super) dataset_metrics: HashMap<&'model JsonPointer, &'model Name>,
    /// The elements E0.d made unusable.
    pub(super) unusable: HashSet<&'model JsonPointer>,
    relationships: &'model Relationships,
}

impl<'model, 'document> Model<'model, 'document> {
    /// What the checks read in the semantic model `outline` that `structure` selected, with
    /// the elements `references` made unusable and the graph of `relationships`.
    pub(super) fn new(
        structure: &'model Structure<'_>,
        outline: &'model ModelOutline<'document>,
        references: &'model References,
        relationships: &'model Relationships,
    ) -> Self {
        Self {
            datasets: named_datasets(outline),
            metrics: structure
                .expressions
                .iter()
                .filter_map(metric_tree)
                .collect(),
            dataset_metrics: dataset_metrics(structure, outline),
            unusable: references
                .unusable
                .iter()
                .map(|element| &element.pointer)
                .collect(),
            relationships,
        }
    }

    /// The names of the datasets a path leads to from `home` (E0E-2.3, E0E-2.4).
    pub(super) fn reached_from(&self, home: &Name) -> HashSet<Name> {
        let graph = &self.relationships.graph;
        let Some(source) = graph
            .datasets
            .iter()
            .position(|dataset| dataset.name == *home)
        else {
            return HashSet::new();
        };
        self.relationships
            .reachable
            .get(source)
            .into_iter()
            .flatten()
            .filter_map(|reached| self.dataset_name(reached.dataset))
            .collect()
    }

    fn dataset_name(&self, index: DatasetIndex) -> Option<Name> {
        self.relationships
            .graph
            .datasets
            .get(index.0)
            .map(|dataset| dataset.name.clone())
    }
}

fn named_datasets<'model, 'document>(
    outline: &'model ModelOutline<'document>,
) -> HashMap<&'model Name, &'model DatasetOutline<'document>> {
    outline
        .datasets
        .iter()
        .filter_map(|dataset| Some((&dataset.name.as_ref()?.name, dataset)))
        .collect()
}

/// Every dataset metric of `outline` that is usable after E0.b, with the name of its dataset.
fn dataset_metrics<'model>(
    structure: &Structure<'_>,
    outline: &'model ModelOutline<'_>,
) -> HashMap<&'model JsonPointer, &'model Name> {
    let usable: HashSet<&JsonPointer> = structure
        .expressions
        .iter()
        .filter(|expression| {
            expression.kind == ElementKind::DatasetMetric && expression.is_usable()
        })
        .map(|expression| &expression.pointer)
        .collect();
    outline
        .datasets
        .iter()
        .filter_map(|dataset| Some((&dataset.name.as_ref()?.name, dataset)))
        .flat_map(|(name, dataset)| {
            dataset
                .metrics
                .iter()
                .map(move |metric| (&metric.pointer, name))
        })
        .filter(|(pointer, _)| usable.contains(pointer))
        .collect()
}

/// The tree of a usable model metric that parses. A metric made unusable in E0.b is left out
/// (E0-4.1), and so is one that does not parse: it has no home dataset.
fn metric_tree<'structure>(
    expression: &'structure ElementExpression<'_>,
) -> Option<(&'structure JsonPointer, Expression)> {
    if expression.kind != ElementKind::Metric || !expression.is_usable() {
        return None;
    }
    let (CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text)) = expression.variant
    else {
        return None;
    };
    let tree = parse_element_expression(text, ExpressionPlace::Metric).ok()?;
    Some((&expression.pointer, tree))
}
