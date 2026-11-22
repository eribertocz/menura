//! `GROUP BY`: exactly the non-aggregated `SELECT` items, by expression or position, or `ALL`
//! (E1-4 "GROUP BY", E1-4.7, E1-4.11).

use std::ops::Range;

use menura_model::expression::Expression;

use super::expression::{SqlExpression, parse_expression};
use super::items::{ItemKind, SelectItem, comparison_name};
use super::lexer::Token;
use super::numbers::integer_literal;
use super::position::{Failure, outside_subset, unparsable};

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

/// A `SELECT` item with its kind and its comparison key (E1-4.11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CheckedItem {
    /// The item as written.
    pub(super) item: SelectItem,
    /// Its kind.
    pub(super) kind: ItemKind,
    /// The tree its expression is compared by.
    pub(super) key: Option<Expression>,
}

/// The `SELECT` items, in order.
pub(super) type Items = [CheckedItem];

/// Checks that `grouping` lists exactly the non-aggregated items of `items`.
///
/// # Errors
///
/// In this order: the first entry that has an E1-4.12 failure, or (`EQRY-05`) is a position
/// outside the items (E1-4.7) or of an aggregated item, or writes no non-aggregated item; then the first non-aggregated
/// item that no entry lists (E1-4.7).
pub(super) fn check_grouping(
    sql: &str,
    grouping: &Grouping,
    items: &Items,
    key: &dyn Fn(&SqlExpression) -> Option<Expression>,
) -> Result<(), Failure> {
    let mut grouped = vec![matches!(grouping, Grouping::All); items.len()];
    if let Grouping::Entries(entries) = grouping {
        for entry in entries {
            let index = grouped_item(sql, entry, items, key)?;
            if let Some(flag) = grouped.get_mut(index) {
                *flag = true;
            }
        }
    }
    let ungrouped = items
        .iter()
        .zip(&grouped)
        .find(|(checked, grouped)| !checked.kind.is_aggregated() && !**grouped);
    match ungrouped {
        Some((CheckedItem { item, .. }, _)) => Err(outside_subset(
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
fn grouped_item(
    sql: &str,
    entry: &GroupEntry,
    items: &Items,
    key: &dyn Fn(&SqlExpression) -> Option<Expression>,
) -> Result<usize, Failure> {
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
                .is_some_and(|checked| checked.kind.is_aggregated())
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
            expression.tree()?;
            non_aggregated_match(key(expression).as_ref(), items).ok_or_else(|| {
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

/// The index of the non-aggregated item whose comparison key is `key` (E1-4.11). Never by
/// output name.
pub(super) fn non_aggregated_match(key: Option<&Expression>, items: &Items) -> Option<usize> {
    let key = key?;
    items
        .iter()
        .position(|checked| !checked.kind.is_aggregated() && checked.key.as_ref() == Some(key))
}

/// The name of an expression that is one unqualified identifier, as E0D-1.1 compares it.
pub(super) fn bare_name(expression: &SqlExpression) -> Option<String> {
    match expression.tree().ok()? {
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
/// `EQRY-05` when an entry does not parse, or `ALL` is written with other entries. The text
/// is read up to byte `end`.
pub(super) fn grouping(
    sql: &str,
    tokens: &[Token],
    entries: Option<Vec<Range<usize>>>,
    end: usize,
) -> Result<Grouping, Failure> {
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
                None => GroupEntry::Expression(parse_expression(sql, tokens, range, end)?),
            })
        })
        .collect::<Result<_, _>>()
        .map(Grouping::Entries)
}
