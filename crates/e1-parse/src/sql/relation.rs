//! The `FROM` relation: exactly one `<model>.<dataset> [[AS] alias]` (E1-4 "FROM", QRY-5.1).

use menura_model::expression::Identifier;
use menura_representations::query_tree::QueryRelation;

use super::clauses::Clause;
use super::lexer::{Token, TokenKind};
use super::position::{location, outside_subset, unparsable};
use crate::error::ParseError;

/// Reads the relation of the `FROM` clause.
///
/// # Errors
///
/// `EQRY-05` for a second relation, a parenthesized relation, a name that is not
/// `<model>.<dataset>`, or text after the alias.
pub(super) fn relation(
    sql: &str,
    tokens: &[Token],
    clause: &Clause,
) -> Result<QueryRelation, ParseError> {
    let slice = tokens.get(clause.tokens.clone()).unwrap_or_default();
    if let Some(comma) = slice.iter().find(|token| token.is_symbol(',')) {
        return Err(outside_subset(
            sql,
            comma.start,
            "a second relation in FROM",
        ));
    }
    if let Some(first) = slice.first().filter(|token| token.is_symbol('(')) {
        return Err(outside_subset(
            sql,
            first.start,
            "a parenthesized relation in FROM",
        ));
    }
    let (model, dataset, name_length) = relation_name(sql, slice)?;
    let alias = relation_alias(sql, slice.get(name_length..).unwrap_or_default())?;
    Ok(QueryRelation {
        model,
        dataset,
        alias,
        origin: location(sql, slice.first().map_or(sql.len(), |token| token.start)),
    })
}

/// The model and dataset names that `slice` starts with, and the number of their tokens.
fn relation_name(
    sql: &str,
    slice: &[Token],
) -> Result<(Identifier, Identifier, usize), ParseError> {
    let mut parts = Vec::new();
    let mut index = 0;
    while let Some(part) = slice.get(index).and_then(|token| identifier(sql, token)) {
        parts.push(part);
        index += 1;
        if !slice.get(index).is_some_and(|token| token.is_symbol('.')) {
            break;
        }
        index += 1;
    }
    let start = slice.first().map_or(sql.len(), |token| token.start);
    let [model, dataset] = <[Identifier; 2]>::try_from(parts).map_err(|_| {
        outside_subset(
            sql,
            start,
            "a FROM relation that is not `<model>.<dataset>`",
        )
    })?;
    Ok((model, dataset, index))
}

/// The alias of the tokens `rest` after the relation name: `[[AS] alias]`.
fn relation_alias(sql: &str, rest: &[Token]) -> Result<Option<Identifier>, ParseError> {
    let rest = match rest {
        [keyword, after @ ..] if keyword.is_keyword(sql, "AS") => {
            if after.is_empty() {
                return Err(unparsable(sql, sql.len(), "expected an alias after AS"));
            }
            after
        }
        _ => rest,
    };
    match rest {
        [] => Ok(None),
        [token] => identifier(sql, token)
            .map(Some)
            .ok_or_else(|| unparsable(sql, token.start, "expected an alias")),
        [_, extra, ..] => Err(unparsable(sql, extra.start, "expected the end of FROM")),
    }
}

/// The identifier `token` writes, regular or delimited.
pub(super) fn identifier(sql: &str, token: &Token) -> Option<Identifier> {
    let text = token.text(sql);
    match token.kind {
        TokenKind::Word => Identifier::regular(text),
        TokenKind::QuotedIdentifier => text
            .get(1..text.len().saturating_sub(1))
            .and_then(|inner| Identifier::delimited(&inner.replace("\"\"", "\""))),
        _ => None,
    }
}
