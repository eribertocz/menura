//! `ORDER BY` and `LIMIT` (E1-4 "ORDER BY", "LIMIT", E1-4.10).

use std::ops::Range;

use menura_contracts::types::query::{Direction, NullsOrder};
use menura_representations::names::OutputName;
use menura_representations::query_tree::{QueryLimit, QueryOrderItem};

use super::expression::{SqlExpression, parse_expression};
use super::grouping::{Items, bare_name, non_aggregated_match};
use super::items::{comparison_name, output_identifier};
use super::lexer::Token;
use super::numbers::integer_literal;
use super::position::{location, outside_subset, unparsable};
use crate::error::ParseError;

/// An `ORDER BY` item as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OrderEntry {
    /// What it orders by.
    target: OrderTarget,
    /// Its direction: `ASC` when not written (E1-4.10).
    direction: Direction,
    /// Where NULLs go: `NULLS LAST` when not written, in either direction (E1-4.10).
    nulls: NullsOrder,
    /// The offset of its first byte.
    offset: usize,
}

/// What an `ORDER BY` item orders by, as written.
#[derive(Debug, Clone, PartialEq, Eq)]
enum OrderTarget {
    /// A position, from 1.
    Position(u64),
    /// An output name or an expression.
    Expression(SqlExpression),
}

/// Reads the `ORDER BY` item made of the tokens `range`, with the defaults of E1-4.10.
///
/// # Errors
///
/// `EQRY-05` when the item does not parse.
pub(super) fn order_entry(
    sql: &str,
    tokens: &[Token],
    range: Range<usize>,
) -> Result<OrderEntry, ParseError> {
    let offset = tokens
        .get(range.start)
        .map_or(sql.len(), |token| token.start);
    let mut end = range.end;
    let word_at = |index: Option<usize>, word: &str| {
        index
            .and_then(|index| tokens.get(index))
            .filter(|_| index.is_some_and(|index| index >= range.start))
            .is_some_and(|token| token.is_keyword(sql, word))
    };
    let mut nulls = NullsOrder::Last;
    if word_at(end.checked_sub(2), "NULLS") {
        nulls = if word_at(end.checked_sub(1), "FIRST") {
            NullsOrder::First
        } else if word_at(end.checked_sub(1), "LAST") {
            NullsOrder::Last
        } else {
            let at = tokens.get(end - 1).map_or(sql.len(), |token| token.start);
            return Err(unparsable(sql, at, "expected FIRST or LAST after NULLS"));
        };
        end -= 2;
    }
    let mut direction = Direction::Asc;
    if word_at(end.checked_sub(1), "DESC") {
        direction = Direction::Desc;
        end -= 1;
    } else if word_at(end.checked_sub(1), "ASC") {
        end -= 1;
    }
    let target = match integer_literal(sql, tokens, range.start..end) {
        Some(position) => OrderTarget::Position(position),
        None => OrderTarget::Expression(parse_expression(sql, tokens, range.start..end)?),
    };
    Ok(OrderEntry {
        target,
        direction,
        nulls,
        offset,
    })
}

/// The ordering term of `entry`: the output it sorts by.
///
/// # Errors
///
/// `EQRY-05` for an item that is no output name, no position of an item, and no expression
/// written as a non-aggregated item.
pub(super) fn order_item(
    sql: &str,
    entry: &OrderEntry,
    items: &Items,
) -> Result<QueryOrderItem, ParseError> {
    let index = match &entry.target {
        OrderTarget::Position(position) => usize::try_from(*position)
            .ok()
            .and_then(|position| position.checked_sub(1))
            .filter(|&index| index < items.len()),
        OrderTarget::Expression(expression) => {
            output_match(expression, items).or_else(|| non_aggregated_match(expression, items))
        }
    };
    let output = index
        .and_then(|index| items.get(index))
        .and_then(|(item, kind)| output_identifier(item, kind));
    let Some(output) = output else {
        return Err(outside_subset(
            sql,
            entry.offset,
            "an ORDER BY item that is no output name, position or non-aggregated SELECT item",
        ));
    };
    Ok(QueryOrderItem {
        output: OutputName::new(output.text()),
        direction: entry.direction,
        nulls: entry.nulls,
        origin: location(sql, entry.offset),
    })
}

/// The index of the item whose output name `expression` writes.
fn output_match(expression: &SqlExpression, items: &Items) -> Option<usize> {
    let name = bare_name(expression)?;
    items.iter().position(|(item, kind)| {
        output_identifier(item, kind).map(comparison_name).as_ref() == Some(&name)
    })
}

/// The `LIMIT` of the tokens `range`: a positive integer literal (E1-4 "LIMIT").
///
/// # Errors
///
/// `EQRY-05` for anything else.
pub(super) fn limit(
    sql: &str,
    tokens: &[Token],
    range: Range<usize>,
) -> Result<QueryLimit, ParseError> {
    let offset = tokens
        .get(range.start)
        .map_or(sql.len(), |token| token.start);
    match integer_literal(sql, tokens, range) {
        Some(rows) if rows > 0 => Ok(QueryLimit {
            rows,
            origin: location(sql, offset),
        }),
        _ => Err(outside_subset(
            sql,
            offset,
            "a LIMIT that is not a positive integer literal",
        )),
    }
}
