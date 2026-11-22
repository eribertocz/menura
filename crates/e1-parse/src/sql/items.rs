//! The `SELECT` items: their expression, alias and kind (E1-4 "SELECT items", E1-4.8,
//! `EQRY-13`).

use std::ops::Range;

use menura_model::expression::{Expression, Identifier, Reference};

use super::expression::{SqlExpression, parse_expression};
use super::lexer::Token;
use super::position::{missing_alias, outside_subset, unparsable};
use super::relation::identifier;
use super::shape::contents;
use crate::error::ParseError;

/// A `SELECT` item as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SelectItem {
    /// Its expression.
    pub(super) expression: SqlExpression,
    /// Its alias, when written with `AS`.
    pub(super) alias: Option<Identifier>,
}

/// What a `SELECT` item is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ItemKind {
    /// A plain field reference: a dimension.
    Field(Reference),
    /// A scalar expression over fields: a dimension.
    Scalar,
    /// `AGG(metric)` alone: a metric reference.
    Metric(Reference),
    /// A scalar expression over `AGG(…)` terms: an inline metric.
    MetricExpression,
}

impl ItemKind {
    /// Whether the item is an aggregated one, a metric.
    pub(super) fn is_aggregated(&self) -> bool {
        matches!(self, Self::Metric(_) | Self::MetricExpression)
    }
}

/// Reads the `SELECT` item made of the tokens `range`: `expression [AS alias]`.
///
/// # Errors
///
/// `EQRY-05` for `*`, a malformed alias, or an expression that does not parse or is outside
/// the subset.
pub(super) fn select_item(
    sql: &str,
    tokens: &[Token],
    range: Range<usize>,
) -> Result<SelectItem, ParseError> {
    let slice = tokens.get(range.clone()).unwrap_or_default();
    if let [only] = slice
        && only.is_symbol('*')
    {
        return Err(outside_subset(sql, only.start, "SELECT *"));
    }
    let (expression_range, alias) = match slice {
        [.., keyword, name] if slice.len() >= 3 && keyword.is_keyword(sql, "AS") => {
            (range.start..range.end - 2, Some(alias(sql, name)?))
        }
        _ => (range, None),
    };
    Ok(SelectItem {
        expression: parse_expression(sql, tokens, expression_range)?,
        alias,
    })
}

/// The identifier `token` written after `AS`.
fn alias(sql: &str, token: &Token) -> Result<Identifier, ParseError> {
    identifier(sql, token).ok_or_else(|| unparsable(sql, token.start, "expected an alias after AS"))
}

/// The kind of `item`.
///
/// # Errors
///
/// `EQRY-05` for an aggregate function or `USER_ATTRIBUTE`, or an item that mixes field
/// references with `AGG(…)` terms (E1-4.8); then `EQRY-13` for an item that is not a plain
/// field reference and has no alias.
pub(super) fn item_kind(sql: &str, item: &SelectItem) -> Result<ItemKind, ParseError> {
    let expression = &item.expression;
    let found = contents(&expression.tree);
    if let Some(construct) = found.unsupported {
        return Err(outside_subset(sql, expression.start, construct));
    }
    if found.metric_aggregate && found.field_reference {
        return Err(outside_subset(
            sql,
            expression.start,
            &format!(
                "the SELECT item `{}`, which mixes field references with AGG(…) terms,",
                expression.text
            ),
        ));
    }
    let kind = match (&expression.tree, found.metric_aggregate) {
        (Expression::MetricAggregate(reference), _) => ItemKind::Metric(reference.clone()),
        (_, true) => ItemKind::MetricExpression,
        (Expression::Reference(reference), false) => ItemKind::Field(reference.clone()),
        (_, false) => ItemKind::Scalar,
    };
    if item.alias.is_none() && !matches!(kind, ItemKind::Field(_)) {
        return Err(missing_alias(sql, expression.start, &expression.text));
    }
    Ok(kind)
}

/// The output name of an item: its alias as written, or else the name of its field reference
/// (E1-5.1).
pub(super) fn output_identifier<'item>(
    item: &'item SelectItem,
    kind: &'item ItemKind,
) -> Option<&'item Identifier> {
    item.alias.as_ref().or(match kind {
        ItemKind::Field(reference) => Some(&reference.name),
        _ => None,
    })
}

/// The name `identifier` as E0D-1.1 compares it: upper case when regular, exact when
/// delimited.
pub(super) fn comparison_name(identifier: &Identifier) -> String {
    if identifier.is_delimited() {
        identifier.text().to_owned()
    } else {
        identifier.text().to_uppercase()
    }
}
