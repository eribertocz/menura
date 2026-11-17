//! The lookups every reference check shares: datasets and fields of the model by name, columns
//! of the catalog relations, and the parsed expression of each field and metric.

use std::collections::HashMap;

use semantic_compiler_contracts::types::catalog::CatalogSchemaDocument;
use semantic_compiler_model::expression::Expression;

use super::columns::Columns;
use super::outline::{DatasetOutline, ElementOutline, ModelOutline};
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::names::Name;
use crate::pointer::JsonPointer;
use crate::structure::{CompiledVariant, ElementExpression, ElementKind};

/// What the checks look names up in.
pub(crate) struct Scope<'model, 'document> {
    /// The outline of the selected semantic model.
    pub(crate) model: &'model ModelOutline<'document>,
    /// The first dataset of each name.
    datasets: HashMap<&'model Name, &'model DatasetOutline<'document>>,
    /// The columns of each catalog relation, by `source` text.
    relations: HashMap<&'model str, Columns<'model>>,
    /// The tree of each field, dataset metric and model metric whose compiled variant parses
    /// (E0.b: the single `ANSI_SQL` variant, or the development-line `sql`), by element pointer.
    /// Elements whose expression cannot be used are left out: E0.b and E0.f report them.
    trees: HashMap<&'model JsonPointer, Expression>,
}

impl<'model, 'document> Scope<'model, 'document> {
    /// The scope of `model`, with the relations of `catalog` and the expressions of
    /// `expressions`.
    pub(crate) fn new(
        model: &'model ModelOutline<'document>,
        catalog: &'model CatalogSchemaDocument,
        expressions: &'model [ElementExpression<'document>],
    ) -> Self {
        let mut datasets = HashMap::new();
        for dataset in &model.datasets {
            if let Some(name) = &dataset.name {
                datasets.entry(&name.name).or_insert(dataset);
            }
        }
        let relations = catalog
            .relations
            .iter()
            .map(|relation| (relation.source.as_str(), Columns::new(relation)))
            .collect();
        let trees = expressions.iter().filter_map(parsed).collect();
        Self {
            model,
            datasets,
            relations,
            trees,
        }
    }

    /// The first dataset named `name`.
    pub(crate) fn dataset(&self, name: &Name) -> Option<&'model DatasetOutline<'document>> {
        self.datasets.get(name).copied()
    }

    /// Whether the catalog schema describes a relation with exactly this `source` text.
    pub(crate) fn describes(&self, source: &str) -> bool {
        self.relations.contains_key(source)
    }

    /// The column names of the catalog relation of `dataset`, if the catalog describes its
    /// `source`.
    pub(crate) fn columns(&self, dataset: &DatasetOutline<'_>) -> Option<&Columns<'model>> {
        self.relations.get(dataset.source.as_ref()?.text)
    }

    /// The parsed expression of the field or metric at `pointer`, if it can be used.
    pub(crate) fn tree(&self, pointer: &JsonPointer) -> Option<&Expression> {
        self.trees.get(pointer)
    }
}

/// The first field of `dataset` named `name`.
pub(crate) fn field<'model, 'document>(
    dataset: &'model DatasetOutline<'document>,
    name: &Name,
) -> Option<&'model ElementOutline<'document>> {
    dataset
        .fields
        .iter()
        .find(|field| field.name.as_ref().is_some_and(|field| field.name == *name))
}

fn parsed<'model>(
    expression: &'model ElementExpression<'_>,
) -> Option<(&'model JsonPointer, Expression)> {
    let text = match expression.variant {
        CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text) => text,
        CompiledVariant::Unusable => return None,
    };
    let place = match expression.kind {
        ElementKind::Field => ExpressionPlace::Field,
        ElementKind::Metric | ElementKind::DatasetMetric => ExpressionPlace::Metric,
    };
    let tree = parse_element_expression(text, place).ok()?;
    Some((&expression.pointer, tree))
}
