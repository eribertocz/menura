//! Query trees of the SQL form, as E1 would build them (E1-4, E1-5).

use menura_model::expression::{Expression, Identifier, ScalarFunction};
use menura_representations::columns::OutputColumn;
use menura_representations::names::OutputName;
use menura_representations::origin::{QueryLocation, TextPosition};
use menura_representations::query_tree::{
    QueryDimension, QueryDimensionKind, QueryFilter, QueryMetric, QueryMetricKind, QueryRelation,
    QueryTree, QueryTreeParts,
};

use super::query::{reference, utc};

/// One output column of a SQL-form tree.
pub type SqlColumn = OutputColumn<QueryDimension, QueryMetric>;

/// The location `/sql`, line 1, at `column`.
pub fn sql_at(column: usize) -> QueryLocation {
    QueryLocation::new("/sql", TextPosition::new(1, column))
}

fn identifier(text: &str) -> Identifier {
    Identifier::regular(text).expect("an identifier")
}

/// The relation `model.dataset [AS alias]`, written at `column`.
pub fn relation(model: &str, dataset: &str, alias: Option<&str>, column: usize) -> QueryRelation {
    QueryRelation {
        model: identifier(model),
        dataset: identifier(dataset),
        alias: alias.map(identifier),
        origin: sql_at(column),
    }
}

/// The plain field item `text`, written at `column`.
pub fn field_item(text: &str, column: usize) -> SqlColumn {
    let reference = reference(text);
    OutputColumn::Dimension(QueryDimension {
        output_name: OutputName::from_reference(&reference),
        kind: QueryDimensionKind::Field { reference },
        origin: sql_at(column),
    })
}

/// The scalar item `expression AS alias`, written at `column`.
pub fn expression_item(expression: Expression, alias: &str, column: usize) -> SqlColumn {
    OutputColumn::Dimension(QueryDimension {
        output_name: OutputName::new(alias),
        kind: QueryDimensionKind::Expression(expression),
        origin: sql_at(column),
    })
}

/// The item `AGG(text) AS alias`, written at `column`.
pub fn agg_item(text: &str, alias: &str, column: usize) -> SqlColumn {
    OutputColumn::Metric(QueryMetric {
        output_name: OutputName::new(alias),
        kind: QueryMetricKind::Reference(reference(text)),
        origin: sql_at(column),
    })
}

/// The item `expression AS alias` over `AGG(…)` terms, written at `column`.
pub fn inline_item(expression: Expression, alias: &str, column: usize) -> SqlColumn {
    OutputColumn::Metric(QueryMetric {
        output_name: OutputName::new(alias),
        kind: QueryMetricKind::Inline(expression),
        origin: sql_at(column),
    })
}

/// The `WHERE` filter `expression`, written at `column`.
pub fn where_filter(expression: Expression, column: usize) -> QueryFilter {
    QueryFilter {
        expression,
        origin: sql_at(column),
    }
}

/// `UPPER(argument)`.
pub fn upper(argument: Expression) -> Expression {
    Expression::Function {
        function: ScalarFunction::Upper,
        arguments: vec![argument],
    }
}

/// The SQL-form tree over `relation` with `columns` in `SELECT` order and `parts`.
pub fn sql_tree(
    relation: QueryRelation,
    columns: Vec<SqlColumn>,
    parts: QueryTreeParts,
) -> QueryTree {
    QueryTree::in_select_order(relation, columns, utc(), parts)
}

/// The SQL-form tree over `relation` with `columns` only.
pub fn sql_columns(relation: QueryRelation, columns: Vec<SqlColumn>) -> QueryTree {
    sql_tree(relation, columns, QueryTreeParts::default())
}

/// The SQL-form tree over `relation` with `columns` and the one `WHERE` filter `filter`.
pub fn sql_where(
    relation: QueryRelation,
    columns: Vec<SqlColumn>,
    filter: QueryFilter,
) -> QueryTree {
    sql_tree(
        relation,
        columns,
        QueryTreeParts {
            filters: vec![filter],
            ..QueryTreeParts::default()
        },
    )
}
