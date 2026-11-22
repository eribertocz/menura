//! `ORDER BY` and `LIMIT` (E1-4 "ORDER BY", "LIMIT", E1-4.10).

use std::ops::Range;

use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::Expression;
use menura_representations::names::OutputName;
use menura_representations::query_tree::{QueryLimit, QueryOrderItem};

use super::clauses::next_offset;
use super::expression::{SqlExpression, parse_expression};
use super::grouping::{Items, bare_name, non_aggregated_match};
use super::items::{comparison_name, output_identifier};
use super::lexer::Token;
use super::numbers::integer_literal;
use super::position::{Failure, location, outside_subset, unparsable};

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
/// `EQRY-05` when the item does not parse. The text is read up to byte `end`.
pub(super) fn order_entry(
    sql: &str,
    tokens: &[Token],
    range: Range<usize>,
    end: usize,
) -> Result<OrderEntry, Failure> {
    let offset = next_offset(tokens, range.start, end);
    let mut stop = range.end;
    let word_at = |index: Option<usize>, word: &str| {
        index
            .and_then(|index| tokens.get(index))
            .filter(|_| index.is_some_and(|index| index >= range.start))
            .is_some_and(|token| token.is_keyword(sql, word))
    };
    let mut nulls = NullsOrder::Last;
    if word_at(stop.checked_sub(2), "NULLS") {
        nulls = if word_at(stop.checked_sub(1), "FIRST") {
            NullsOrder::First
        } else if word_at(stop.checked_sub(1), "LAST") {
            NullsOrder::Last
        } else {
            let at = next_offset(tokens, stop - 1, end);
            return Err(unparsable(sql, at, "expected FIRST or LAST after NULLS"));
        };
        stop -= 2;
    }
    let mut direction = Direction::Asc;
    if word_at(stop.checked_sub(1), "DESC") {
        direction = Direction::Desc;
        stop -= 1;
    } else if word_at(stop.checked_sub(1), "ASC") {
        stop -= 1;
    }
    let target = match integer_literal(sql, tokens, range.start..stop) {
        Some(position) => OrderTarget::Position(position),
        None => OrderTarget::Expression(parse_expression(sql, tokens, range.start..stop, end)?),
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
/// The E1-4.12 failure of its expression; `EQRY-05` for an item that is no output name, no position of an item, and no expression
/// written as a non-aggregated item.
pub(super) fn order_item(
    sql: &str,
    entry: &OrderEntry,
    items: &Items,
    key: &dyn Fn(&SqlExpression) -> Option<Expression>,
) -> Result<QueryOrderItem, Failure> {
    let index = match &entry.target {
        OrderTarget::Position(position) => usize::try_from(*position)
            .ok()
            .and_then(|position| position.checked_sub(1))
            .filter(|&index| index < items.len()),
        OrderTarget::Expression(expression) => {
            expression.tree()?;
            output_match(expression, items)
                .or_else(|| non_aggregated_match(key(expression).as_ref(), items))
        }
    };
    let output = index
        .and_then(|index| items.get(index))
        .and_then(|checked| output_identifier(&checked.item, &checked.kind));
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
    items.iter().position(|checked| {
        output_identifier(&checked.item, &checked.kind)
            .map(comparison_name)
            .as_ref()
            == Some(&name)
    })
}

/// The largest `LIMIT`, 2^63 − 1 (E1-4.11).
const MAXIMUM_LIMIT: u64 = 9_223_372_036_854_775_807;

/// The `LIMIT` of the tokens `range`: a positive integer literal up to [`MAXIMUM_LIMIT`]
/// (E1-4 "LIMIT", E1-4.11).
///
/// # Errors
///
/// `EQRY-05` for anything else.
pub(super) fn limit(
    sql: &str,
    tokens: &[Token],
    range: Range<usize>,
) -> Result<QueryLimit, Failure> {
    let offset = next_offset(tokens, range.start, sql.len());
    match integer_literal(sql, tokens, range) {
        Some(rows) if rows > 0 && rows <= MAXIMUM_LIMIT => Ok(QueryLimit {
            rows,
            origin: location(sql, offset),
        }),
        _ => Err(outside_subset(
            sql,
            offset,
            "a LIMIT that is not a positive integer literal up to 2^63 - 1",
        )),
    }
}
