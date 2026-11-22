//! The clauses of a SQL statement, found at parenthesis depth 0, and the statement-level
//! constructs outside the version 1 subset (E1-4 "Statement").

use std::ops::Range;

use super::lexer::Token;
use super::position::{outside_subset, unparsable};
use crate::error::ParseError;

/// A clause: the tokens after its keyword, as a range of token indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Clause {
    /// The tokens of its body, never empty.
    pub(super) tokens: Range<usize>,
}

/// The clauses of the one `SELECT` statement, in the order the subset allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Statement {
    /// `SELECT` items.
    pub(super) select: Clause,
    /// `FROM` relation.
    pub(super) from: Clause,
    /// `WHERE` condition.
    pub(super) filter: Option<Clause>,
    /// `GROUP BY` entries.
    pub(super) group_by: Option<Clause>,
    /// `ORDER BY` items.
    pub(super) order_by: Option<Clause>,
    /// `LIMIT` count.
    pub(super) limit: Option<Clause>,
}

/// The clause keywords, in the only order the subset allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ClauseKeyword {
    Select,
    From,
    Where,
    GroupBy,
    OrderBy,
    Limit,
}

impl ClauseKeyword {
    fn name(self) -> &'static str {
        match self {
            Self::Select => "SELECT",
            Self::From => "FROM",
            Self::Where => "WHERE",
            Self::GroupBy => "GROUP BY",
            Self::OrderBy => "ORDER BY",
            Self::Limit => "LIMIT",
        }
    }
}

/// Keywords at depth 0 that start a construct outside the subset, and how it is named.
const UNSUPPORTED_KEYWORDS: [(&str, &str); 10] = [
    ("WITH", "WITH"),
    ("UNION", "the set operator UNION"),
    ("INTERSECT", "the set operator INTERSECT"),
    ("EXCEPT", "the set operator EXCEPT"),
    ("JOIN", "JOIN"),
    ("HAVING", "HAVING"),
    ("QUALIFY", "QUALIFY"),
    ("WINDOW", "WINDOW"),
    ("OFFSET", "OFFSET"),
    ("FETCH", "FETCH"),
];

/// The clauses of the statement made of `tokens`: one `SELECT`, with an optional trailing `;`.
///
/// # Errors
///
/// `EQRY-05` at the first token, in text order, that breaks the statement's structure or
/// starts a construct outside the subset.
pub(super) fn statement(sql: &str, tokens: &[Token]) -> Result<Statement, ParseError> {
    let count = match tokens.last() {
        Some(last) if last.is_symbol(';') => tokens.len() - 1,
        _ => tokens.len(),
    };
    let starts = clause_starts(sql, &tokens[..count])?;
    let mut statement = Statement {
        select: Clause { tokens: 0..0 },
        from: Clause { tokens: 0..0 },
        filter: None,
        group_by: None,
        order_by: None,
        limit: None,
    };
    let mut has_from = false;
    for (index, &(keyword, body_start)) in starts.iter().enumerate() {
        let body_end = starts.get(index + 1).map_or(count, |&(next, next_start)| {
            next_start - keyword_length(next)
        });
        if body_start >= body_end {
            let offset = tokens
                .get(body_start)
                .map_or(sql.len(), |token| token.start);
            return Err(unparsable(
                sql,
                offset,
                &format!("expected the body of {}", keyword.name()),
            ));
        }
        let clause = Clause {
            tokens: body_start..body_end,
        };
        match keyword {
            ClauseKeyword::Select => statement.select = clause,
            ClauseKeyword::From => {
                has_from = true;
                statement.from = clause;
            }
            ClauseKeyword::Where => statement.filter = Some(clause),
            ClauseKeyword::GroupBy => statement.group_by = Some(clause),
            ClauseKeyword::OrderBy => statement.order_by = Some(clause),
            ClauseKeyword::Limit => statement.limit = Some(clause),
        }
    }
    if !has_from {
        return Err(unparsable(sql, sql.len(), "expected FROM"));
    }
    Ok(statement)
}

/// The number of tokens of the clause keyword.
fn keyword_length(keyword: ClauseKeyword) -> usize {
    match keyword {
        ClauseKeyword::GroupBy | ClauseKeyword::OrderBy => 2,
        _ => 1,
    }
}

/// Each clause keyword of `tokens`, in order, with the index of the first token of its body.
fn clause_starts(sql: &str, tokens: &[Token]) -> Result<Vec<(ClauseKeyword, usize)>, ParseError> {
    let Some(first) = tokens.first() else {
        return Err(unparsable(sql, sql.len(), "expected SELECT"));
    };
    if first.is_keyword(sql, "WITH") {
        return Err(outside_subset(sql, first.start, "WITH"));
    }
    if !first.is_keyword(sql, "SELECT") {
        return Err(unparsable(sql, first.start, "expected SELECT"));
    }
    let mut starts = Vec::new();
    let mut open_parentheses = Vec::new();
    let mut index = 0;
    while let Some(token) = tokens.get(index) {
        if token.is_symbol('(') {
            open_parentheses.push(token.start);
        } else if token.is_symbol(')') {
            if open_parentheses.pop().is_none() {
                return Err(unparsable(sql, token.start, "`)` closes no parenthesis"));
            }
        } else if token.is_symbol(';') {
            return Err(outside_subset(sql, token.start, "a second statement"));
        } else if token.is_keyword(sql, "SELECT") && !open_parentheses.is_empty() {
            return Err(outside_subset(sql, token.start, "a subquery"));
        } else if open_parentheses.is_empty() {
            if token.is_keyword(sql, "DISTINCT") {
                index += distinct_length(sql, tokens, index)?;
                continue;
            }
            if let Some(keyword) = clause_keyword(sql, tokens, index)? {
                if starts.last().is_some_and(|&(last, _)| last >= keyword) {
                    return Err(unparsable(
                        sql,
                        token.start,
                        &format!("unexpected {}", keyword.name()),
                    ));
                }
                index += keyword_length(keyword);
                starts.push((keyword, index));
                continue;
            }
            check_supported(sql, token)?;
        }
        index += 1;
    }
    match open_parentheses.last() {
        Some(&offset) => Err(unparsable(sql, offset, "`(` is never closed")),
        None => Ok(starts),
    }
}

/// The clause keyword that starts at `tokens[index]`, if any.
fn clause_keyword(
    sql: &str,
    tokens: &[Token],
    index: usize,
) -> Result<Option<ClauseKeyword>, ParseError> {
    let Some(token) = tokens.get(index) else {
        return Ok(None);
    };
    let simple = [
        ("SELECT", ClauseKeyword::Select),
        ("FROM", ClauseKeyword::From),
        ("WHERE", ClauseKeyword::Where),
        ("LIMIT", ClauseKeyword::Limit),
    ];
    if let Some(&(_, keyword)) = simple.iter().find(|(word, _)| token.is_keyword(sql, word)) {
        return Ok(Some(keyword));
    }
    let with_by = [
        ("GROUP", ClauseKeyword::GroupBy),
        ("ORDER", ClauseKeyword::OrderBy),
    ];
    let Some(&(word, keyword)) = with_by.iter().find(|(word, _)| token.is_keyword(sql, word))
    else {
        return Ok(None);
    };
    match tokens.get(index + 1) {
        Some(next) if next.is_keyword(sql, "BY") => Ok(Some(keyword)),
        next => Err(unparsable(
            sql,
            next.map_or(sql.len(), |next| next.start),
            &format!("expected BY after {word}"),
        )),
    }
}

/// The number of tokens to step over at the `DISTINCT` of `tokens[index]`, at depth 0: two
/// in `IS [NOT] DISTINCT FROM`, whose `FROM` starts no clause, and one elsewhere.
///
/// # Errors
///
/// `EQRY-05` for `SELECT DISTINCT`.
fn distinct_length(sql: &str, tokens: &[Token], index: usize) -> Result<usize, ParseError> {
    let before = |distance: usize| index.checked_sub(distance).and_then(|at| tokens.get(at));
    let is_word =
        |token: Option<&Token>, word: &str| token.is_some_and(|token| token.is_keyword(sql, word));
    if is_word(before(1), "SELECT") {
        let offset = tokens.get(index).map_or(sql.len(), |token| token.start);
        return Err(outside_subset(sql, offset, "SELECT DISTINCT"));
    }
    let in_comparison =
        is_word(before(1), "IS") || (is_word(before(1), "NOT") && is_word(before(2), "IS"));
    let followed_by_from = is_word(tokens.get(index + 1), "FROM");
    Ok(if in_comparison && followed_by_from {
        2
    } else {
        1
    })
}

/// `EQRY-05` when `token`, at depth 0, starts a construct outside the subset.
fn check_supported(sql: &str, token: &Token) -> Result<(), ParseError> {
    match UNSUPPORTED_KEYWORDS
        .iter()
        .find(|(word, _)| token.is_keyword(sql, word))
    {
        Some((_, construct)) => Err(outside_subset(sql, token.start, construct)),
        None => Ok(()),
    }
}

/// The ranges of the items of `clause`, separated by commas at depth 0.
///
/// # Errors
///
/// `EQRY-05` at a comma with no item before it, or at the end when the last item is empty.
pub(super) fn items(
    sql: &str,
    tokens: &[Token],
    clause: &Clause,
) -> Result<Vec<Range<usize>>, ParseError> {
    let mut items = Vec::new();
    let mut depth = 0_usize;
    let mut start = clause.tokens.start;
    for index in clause.tokens.clone() {
        let Some(token) = tokens.get(index) else {
            break;
        };
        if token.is_symbol('(') {
            depth += 1;
        } else if token.is_symbol(')') {
            depth = depth.saturating_sub(1);
        } else if token.is_symbol(',') && depth == 0 {
            if start == index {
                return Err(unparsable(sql, token.start, "expected an item before `,`"));
            }
            items.push(start..index);
            start = index + 1;
        }
    }
    if start == clause.tokens.end {
        let offset = tokens.get(start).map_or(sql.len(), |token| token.start);
        return Err(unparsable(sql, offset, "expected an item"));
    }
    items.push(start..clause.tokens.end);
    Ok(items)
}
