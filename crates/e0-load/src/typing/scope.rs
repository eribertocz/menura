//! What typing looks names up in: datasets of the model by name, the columns of each catalog
//! relation, and the parsed expression of each field and metric.

use std::collections::HashMap;

use semantic_compiler_contracts::types::catalog::{CatalogSchemaDocument, Column, RelationSchema};
use semantic_compiler_model::expression::{Expression, Identifier};

use crate::columns::Columns;
use crate::expressions::{ExpressionPlace, parse_element_expression};
use crate::names::Name;
use crate::outline::{DatasetOutline, ModelOutline};
use crate::pointer::JsonPointer;
use crate::structure::{CompiledVariant, ElementExpression, ElementKind};

/// The lookups every typing check shares.
pub(crate) struct Scope<'model, 'document> {
    /// The outline of the selected semantic model.
    pub(crate) model: &'model ModelOutline<'document>,
    /// The first dataset of each name.
    datasets: HashMap<&'model Name, &'model DatasetOutline<'document>>,
    /// Each catalog relation, by `source` text.
    relations: HashMap<&'model str, Relation<'model>>,
    /// The tree of each field, dataset metric and model metric whose compiled variant parses
    /// (E0.b: the single `ANSI_SQL` variant, or the development-line `sql`), by element pointer.
    /// An expression that does not parse is reported by the parsing of E0.f.
    trees: HashMap<&'model JsonPointer, Expression>,
}

/// The columns of one catalog relation.
pub(crate) struct Relation<'catalog> {
    names: Columns<'catalog>,
    columns: HashMap<&'catalog str, &'catalog Column>,
}

impl<'catalog> Relation<'catalog> {
    fn new(relation: &'catalog RelationSchema) -> Self {
        Self {
            names: Columns::new(relation),
            columns: relation
                .columns
                .iter()
                .map(|column| (column.name.as_str(), column))
                .collect(),
        }
    }

    /// The column a key name matches (E0D-1.5).
    pub(crate) fn find(&self, name: &Name, delimited: bool) -> Option<&'catalog Column> {
        self.names
            .find(name, delimited)
            .and_then(|spelling| self.columns.get(spelling).copied())
    }

    /// The column the expression identifier `identifier` matches (E0D-1.5).
    pub(crate) fn find_identifier(&self, identifier: &Identifier) -> Option<&'catalog Column> {
        self.find(&Name::of(identifier), identifier.is_delimited())
    }
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
        Self {
            model,
            datasets,
            relations: catalog
                .relations
                .iter()
                .map(|relation| (relation.source.as_str(), Relation::new(relation)))
                .collect(),
            trees: expressions.iter().filter_map(parsed).collect(),
        }
    }

    /// The first dataset named `name`.
    pub(crate) fn dataset(&self, name: &Name) -> Option<&'model DatasetOutline<'document>> {
        self.datasets.get(name).copied()
    }

    /// The catalog relation of `dataset`, if the catalog describes its `source`.
    pub(crate) fn relation(&self, dataset: &DatasetOutline<'_>) -> Option<&Relation<'model>> {
        self.relations.get(dataset.source.as_ref()?.text)
    }

    /// The parsed expression of the field or metric at `pointer`, if it parses.
    pub(crate) fn tree(&self, pointer: &JsonPointer) -> Option<&Expression> {
        self.trees.get(pointer)
    }
}

/// The tree of the compiled variant of a field, dataset metric or model metric (E0B-3.3,
/// section 4.5).
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
