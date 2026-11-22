//! `GROUP BY`: exactly the non-aggregated `SELECT` items, by expression, name or position, or
//! `ALL` (E1-4 "GROUP BY", E1-4.7).

use std::ops::Range;

use menura_model::expression::Expression;

use super::expression::{SqlExpression, parse_expression};
use super::items::{ItemKind, SelectItem, comparison_name};
use super::lexer::Token;
use super::numbers::integer_literal;
use super::position::{outside_subset, unparsable};
use crate::error::ParseError;

/// The `GROUP BY` clause as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Grouping {
    /// No `GROUP BY`: the result is one row.
    None,
    /// `GROUP BY ALL`.
    All,
    /// `GROUP BY` entries, in the order written.
    Entries(Vec<GroupEntry>),
}

/// One `GROUP BY` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum GroupEntry {
    /// A position, from 1, as written at byte `offset`.
    Position {
        /// The position, as written.
        position: u64,
        /// The offset of its first byte.
        offset: usize,
    },
    /// An expression written as a non-aggregated item.
    Expression(SqlExpression),
}

/// The `SELECT` items, in order, with their kinds.
pub(super) type Items = [(SelectItem, ItemKind)];

/// Checks that `grouping` lists exactly the non-aggregated items of `items`.
///
/// # Errors
///
/// `EQRY-05`, in this order: the first entry that is a position outside the items (E1-4.7) or
/// of an aggregated item, or that names no non-aggregated item; then the first non-aggregated
/// item that no entry lists (E1-4.7).
pub(super) fn check_grouping(
    sql: &str,
    grouping: &Grouping,
    items: &Items,
) -> Result<(), ParseError> {
    let mut grouped = vec![matches!(grouping, Grouping::All); items.len()];
    if let Grouping::Entries(entries) = grouping {
        for entry in entries {
            let index = grouped_item(sql, entry, items)?;
            if let Some(flag) = grouped.get_mut(index) {
                *flag = true;
            }
        }
    }
    let ungrouped = items
        .iter()
        .zip(&grouped)
        .find(|((_, kind), grouped)| !kind.is_aggregated() && !**grouped);
    match ungrouped {
        Some(((item, _), _)) => Err(outside_subset(
            sql,
            item.expression.start,
            &format!(
                "the non-aggregated SELECT item `{}`, which GROUP BY does not list,",
                item.expression.text
            ),
        )),
        None => Ok(()),
    }
}

/// The index of the non-aggregated item that `entry` lists.
fn grouped_item(sql: &str, entry: &GroupEntry, items: &Items) -> Result<usize, ParseError> {
    match entry {
        GroupEntry::Position { position, offset } => {
            let index = usize::try_from(*position)
                .ok()
                .and_then(|position| position.checked_sub(1))
                .filter(|&index| index < items.len());
            let Some(index) = index else {
                return Err(outside_subset(
                    sql,
                    *offset,
                    &format!(
                        "GROUP BY position {position}, outside the {} SELECT items,",
                        items.len()
                    ),
                ));
            };
            if items
                .get(index)
                .is_some_and(|(_, kind)| kind.is_aggregated())
            {
                return Err(outside_subset(
                    sql,
                    *offset,
                    &format!("GROUP BY position {position}, an aggregated item,"),
                ));
            }
            Ok(index)
        }
        GroupEntry::Expression(expression) => {
            non_aggregated_match(expression, items).ok_or_else(|| {
                outside_subset(
                    sql,
                    expression.start,
                    &format!(
                        "the GROUP BY entry `{}`, which is not a non-aggregated SELECT item,",
                        expression.text
                    ),
                )
            })
        }
    }
}

/// The index of the non-aggregated item that `expression` writes, compared as parsed trees
/// with regular identifiers in upper case (E0D-1.1). Never by output name (E1-4.11).
pub(super) fn non_aggregated_match(expression: &SqlExpression, items: &Items) -> Option<usize> {
    expression.key.as_ref()?;
    items
        .iter()
        .position(|(item, kind)| !kind.is_aggregated() && item.expression.key == expression.key)
}

/// The name of an expression that is one unqualified identifier, as E0D-1.1 compares it.
pub(super) fn bare_name(expression: &SqlExpression) -> Option<String> {
    match &expression.tree {
        Expression::Reference(reference) if reference.qualifier.is_none() => {
            Some(comparison_name(&reference.name))
        }
        _ => None,
    }
}

/// Reads the `GROUP BY` clause made of the item ranges `entries`, or its absence.
///
/// # Errors
///
/// `EQRY-05` when an entry does not parse, or `ALL` is written with other entries.
pub(super) fn grouping(
    sql: &str,
    tokens: &[Token],
    entries: Option<Vec<Range<usize>>>,
) -> Result<Grouping, ParseError> {
    let Some(entries) = entries else {
        return Ok(Grouping::None);
    };
    let is_all = |range: &Range<usize>| match tokens.get(range.clone()) {
        Some([only]) => only.is_keyword(sql, "ALL"),
        _ => false,
    };
    if let [only] = entries.as_slice()
        && is_all(only)
    {
        return Ok(Grouping::All);
    }
    entries
        .into_iter()
        .map(|range| {
            let offset = tokens
                .get(range.start)
                .map_or(sql.len(), |token| token.start);
            if is_all(&range) {
                return Err(unparsable(sql, offset, "GROUP BY ALL takes no other entry"));
            }
            Ok(match integer_literal(sql, tokens, range.clone()) {
                Some(position) => GroupEntry::Position { position, offset },
                None => GroupEntry::Expression(parse_expression(sql, tokens, range)?),
            })
        })
        .collect::<Result<_, _>>()
        .map(Grouping::Entries)
}
