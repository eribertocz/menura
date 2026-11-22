//! The SQL form (E1-4): the version 1 subset of Apache Ossie Layer 2, parsed into the same
//! query tree as the wide-table form (E1-5).
//!
//! The text is read in two passes, which report the first failure in the order of E1-4.11.
//! The first reads its structure: the tokens, the clauses, the relation and every expression,
//! in text order (E1-4.5, the statement-level constructs outside the subset, and E1-4.12). The
//! second checks the items clause by clause: the `SELECT` items in order (E1-4.8, `EQRY-13`),
//! at least one `AGG(…)` item (E1-4.6), `WHERE`, `GROUP BY` (E1-4.7), `ORDER BY`, then
//! `LIMIT`.

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

use std::ops::Range;

use menura_representations::query_tree::{QueryFilter, QueryRelation, QueryTree, QueryTreeParts};
use menura_representations::time::QueryTime;

use crate::error::ParseError;
use clauses::{Clause, ClauseKeyword, items, scan};
use expression::{SqlExpression, comparison_key, parse_expression};
use grouping::{CheckedItem, Grouping, check_grouping, grouping};
use items::{SelectItem, comparison_name, item_kind, select_item};
use lexer::{Token, tokenize};
use ordering::{OrderEntry, limit, order_entry, order_item};
use position::{Failure, location, outside_subset, unparsable};
use relation::relation;
use shape::contents;

/// The statement as written, after the first pass.
struct WrittenQuery {
    items: Vec<SelectItem>,
    relation: QueryRelation,
    filter: Option<SqlExpression>,
    grouping: Grouping,
    order: Vec<OrderEntry>,
    limit: Option<Range<usize>>,
}

/// Parses the SQL text `sql` of a query whose time is `time` into its query tree.
///
/// # Errors
///
/// The first failure in the order of E1-4.11: `EQRY-05` for a text that does not parse
/// (E1-4.5) or a construct outside the subset (E1-4, E1-4.6 to E1-4.8), `E0F-02` for an
/// expression that parses with a construct outside the E0F-2 subset (E1-4.12), and `EQRY-13`
/// for an item without its required alias, all located at `/sql` with the line and column in
/// the message.
pub(crate) fn sql_tree(sql: &str, time: QueryTime) -> Result<QueryTree, ParseError> {
    let (tokens, lexical) = tokenize(sql);
    let end = lexical.map_or(sql.len(), |error| error.offset);
    let lexical = lexical.map(|error| {
        unparsable(
            sql,
            error.offset,
            &format!("the {} is not closed", error.element),
        )
    });
    let written = written_query(sql, &tokens, end, lexical)?;
    checked_tree(sql, &tokens, written, time).map_err(ParseError::from)
}

/// The second pass, in the order of E1-4.11 steps 2 to 7, then the tree.
fn checked_tree(
    sql: &str,
    tokens: &[Token],
    written: WrittenQuery,
    time: QueryTime,
) -> Result<QueryTree, Failure> {
    let qualifiers: Vec<String> = written
        .relation
        .alias
        .iter()
        .chain([&written.relation.dataset])
        .map(comparison_name)
        .collect();
    let key = |expression: &SqlExpression| comparison_key(sql, tokens, expression, &qualifiers);
    let items = written
        .items
        .into_iter()
        .map(|item| {
            let kind = item_kind(sql, &item)?;
            let key = key(&item.expression);
            Ok(CheckedItem { item, kind, key })
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    if !items.iter().any(|checked| checked.kind.is_aggregated()) {
        let start = items
            .first()
            .map_or(0, |checked| checked.item.expression.start);
        return Err(outside_subset(sql, start, "a query with no AGG(…) item"));
    }
    let filters = written
        .filter
        .map(|filter| query_filter(sql, &filter))
        .transpose()?;
    check_grouping(sql, &written.grouping, &items, &key)?;
    let order = written
        .order
        .iter()
        .map(|entry| order_item(sql, entry, &items, &key))
        .collect::<Result<_, _>>()?;
    let limit = written
        .limit
        .map(|range| limit(sql, tokens, range))
        .transpose()?;
    let parts = QueryTreeParts {
        filters: filters.into_iter().collect(),
        order,
        limit,
        ..QueryTreeParts::default()
    };
    Ok(QueryTree::in_select_order(
        written.relation,
        tree::columns(sql, &items)?,
        time,
        parts,
    ))
}

/// The clauses read by the first pass, each once its text has been read without failure.
#[derive(Default)]
struct Read {
    items: Option<Vec<SelectItem>>,
    relation: Option<QueryRelation>,
    filter: Option<SqlExpression>,
    grouping: Option<Grouping>,
    order: Vec<OrderEntry>,
    limit: Option<Range<usize>>,
}

/// The first pass (E1-4.11 step 1): the structure, the relation and every expression. The
/// failure reported is the first in text order: `lexical`, the failures of the structure, and
/// the first failure of each clause, each where a reader going from left to right detects it.
fn written_query(
    sql: &str,
    tokens: &[Token],
    end: usize,
    lexical: Option<Failure>,
) -> Result<WrittenQuery, Failure> {
    let scanned = scan(sql, tokens, end);
    let mut failures: Vec<Failure> = lexical.into_iter().chain(scanned.failures).collect();
    let mut read = Read::default();
    for clause in &scanned.clauses {
        if let Err(failure) = read_clause(sql, tokens, clause, end, &mut read) {
            failures.push(failure);
        }
    }
    if read.relation.is_none()
        && !scanned
            .clauses
            .iter()
            .any(|clause| clause.keyword == ClauseKeyword::From)
    {
        failures.push(unparsable(sql, end, "expected FROM"));
    }
    if let Some(first) = failures.into_iter().min_by_key(|failure| failure.offset) {
        return Err(first);
    }
    match (read.items, read.relation) {
        (Some(items), Some(relation)) => Ok(WrittenQuery {
            items,
            relation,
            filter: read.filter,
            grouping: read.grouping.unwrap_or(Grouping::None),
            order: read.order,
            limit: read.limit,
        }),
        _ => Err(unparsable(sql, end, "expected SELECT and FROM")),
    }
}

/// Reads one clause into `read`.
fn read_clause(
    sql: &str,
    tokens: &[Token],
    clause: &Clause,
    end: usize,
    read: &mut Read,
) -> Result<(), Failure> {
    match clause.keyword {
        ClauseKeyword::Select => {
            read.items = Some(
                items(sql, tokens, clause, end)?
                    .into_iter()
                    .map(|range| select_item(sql, tokens, range, end))
                    .collect::<Result<_, _>>()?,
            );
        }
        ClauseKeyword::From => read.relation = Some(relation(sql, tokens, clause, end)?),
        ClauseKeyword::Where => {
            read.filter = Some(parse_expression(sql, tokens, clause.tokens.clone(), end)?);
        }
        ClauseKeyword::GroupBy => {
            let entries = items(sql, tokens, clause, end)?;
            read.grouping = Some(grouping(sql, tokens, Some(entries), end)?);
        }
        ClauseKeyword::OrderBy => {
            read.order = items(sql, tokens, clause, end)?
                .into_iter()
                .map(|range| order_entry(sql, tokens, range, end))
                .collect::<Result<_, _>>()?;
        }
        ClauseKeyword::Limit => read.limit = Some(clause.tokens.clone()),
    }
    Ok(())
}

/// The `WHERE` condition as a query filter: an expression over fields of the relation.
///
/// # Errors
///
/// The E1-4.12 failure of the condition; `EQRY-05` for an `AGG(…)` term, an aggregate
/// function or `USER_ATTRIBUTE`.
fn query_filter(sql: &str, filter: &SqlExpression) -> Result<QueryFilter, Failure> {
    let tree = filter.tree()?;
    let found = contents(tree);
    if found.metric_aggregate {
        return Err(outside_subset(sql, filter.start, "AGG(…) in WHERE"));
    }
    if let Some(construct) = found.unsupported {
        return Err(outside_subset(sql, filter.start, construct));
    }
    Ok(QueryFilter {
        origin: location(sql, filter.start),
        expression: tree.clone(),
    })
}
