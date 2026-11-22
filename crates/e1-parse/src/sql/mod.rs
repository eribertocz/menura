//! The SQL form (E1-4): the version 1 subset of Apache Ossie Layer 2, parsed into the same
//! query tree as the wide-table form (E1-5).
//!
//! The text is read in two passes. The first reads its structure: the tokens, the clauses,
//! the relation and every expression, in text order (E1-4.5 and the constructs outside the
//! subset). The second checks the items against each other, clause by clause: the `SELECT`
//! items in order (E1-4.8, `EQRY-13`), at least one `AGG(…)` item (E1-4.6), `WHERE`,
//! `GROUP BY` (E1-4.7), then `ORDER BY`.

mod clauses;
mod expression;
mod grouping;
mod items;
mod lexer;
mod numbers;
mod ordering;
mod position;
mod relation;
mod shape;
#[cfg(test)]
mod tests;
mod tree;

use menura_representations::query_tree::{QueryFilter, QueryLimit, QueryTree, QueryTreeParts};
use menura_representations::time::QueryTime;

use crate::error::ParseError;
use clauses::{Statement, items, statement};
use expression::{SqlExpression, parse_expression};
use grouping::{Grouping, check_grouping, grouping};
use items::{ItemKind, SelectItem, item_kind, select_item};
use lexer::{Token, tokenize};
use ordering::{OrderEntry, limit, order_entry, order_item};
use position::{location, outside_subset, unparsable};
use relation::relation;
use shape::contents;

/// The statement as written, after the first pass.
struct WrittenQuery {
    items: Vec<SelectItem>,
    filter: Option<SqlExpression>,
    grouping: Grouping,
    order: Vec<OrderEntry>,
    limit: Option<QueryLimit>,
}

/// Parses the SQL text `sql` of a query whose time is `time` into its query tree.
///
/// # Errors
///
/// `EQRY-05` for a text that does not parse (E1-4.5) or a construct outside the subset (E1-4,
/// E1-4.6 to E1-4.8), and `EQRY-13` for an item without its required alias, all located at
/// `/sql` with the line and column in the message.
pub(crate) fn sql_tree(sql: &str, time: QueryTime) -> Result<QueryTree, ParseError> {
    let tokens = tokenize(sql).map_err(|error| {
        unparsable(
            sql,
            error.offset,
            &format!("the {} is not closed", error.element),
        )
    })?;
    let written = written_query(sql, &tokens)?;
    let items = written
        .items
        .into_iter()
        .map(|item| item_kind(sql, &item).map(|kind| (item, kind)))
        .collect::<Result<Vec<(SelectItem, ItemKind)>, _>>()?;
    if !items.iter().any(|(_, kind)| kind.is_aggregated()) {
        let start = items.first().map_or(0, |(item, _)| item.expression.start);
        return Err(outside_subset(sql, start, "a query with no AGG(…) item"));
    }
    let filters = written
        .filter
        .map(|filter| query_filter(sql, filter))
        .transpose()?;
    check_grouping(sql, &written.grouping, &items)?;
    let order = written
        .order
        .iter()
        .map(|entry| order_item(sql, entry, &items))
        .collect::<Result<_, _>>()?;
    let parts = QueryTreeParts {
        filters: filters.into_iter().collect(),
        order,
        limit: written.limit,
        ..QueryTreeParts::default()
    };
    Ok(QueryTree::in_select_order(
        tree::columns(sql, &items)?,
        time,
        parts,
    ))
}

/// The first pass: the clauses, the relation and every expression, in text order.
fn written_query(sql: &str, tokens: &[Token]) -> Result<WrittenQuery, ParseError> {
    let Statement {
        select,
        from,
        filter,
        group_by,
        order_by,
        limit: limit_clause,
    } = statement(sql, tokens)?;
    let items = items(sql, tokens, &select)?
        .into_iter()
        .map(|range| select_item(sql, tokens, range))
        .collect::<Result<_, _>>()?;
    relation(sql, tokens, &from)?;
    let filter = filter
        .map(|clause| parse_expression(sql, tokens, clause.tokens))
        .transpose()?;
    let group_entries = group_by
        .map(|clause| clauses::items(sql, tokens, &clause))
        .transpose()?;
    let grouping = grouping(sql, tokens, group_entries)?;
    let order = match order_by {
        Some(clause) => clauses::items(sql, tokens, &clause)?
            .into_iter()
            .map(|range| order_entry(sql, tokens, range))
            .collect::<Result<_, _>>()?,
        None => Vec::new(),
    };
    let limit = limit_clause
        .map(|clause| limit(sql, tokens, clause.tokens))
        .transpose()?;
    Ok(WrittenQuery {
        items,
        filter,
        grouping,
        order,
        limit,
    })
}

/// The `WHERE` condition as a query filter: an expression over fields of the relation.
///
/// # Errors
///
/// `EQRY-05` for an `AGG(…)` term, an aggregate function or `USER_ATTRIBUTE`.
fn query_filter(sql: &str, filter: SqlExpression) -> Result<QueryFilter, ParseError> {
    let found = contents(&filter.tree);
    if found.metric_aggregate {
        return Err(outside_subset(sql, filter.start, "AGG(…) in WHERE"));
    }
    if let Some(construct) = found.unsupported {
        return Err(outside_subset(sql, filter.start, construct));
    }
    Ok(QueryFilter {
        origin: location(sql, filter.start),
        expression: filter.tree,
    })
}
