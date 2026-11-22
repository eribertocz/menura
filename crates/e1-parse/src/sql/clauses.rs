//! The clauses of a SQL statement, found at parenthesis depth 0, and the statement-level
//! constructs outside the version 1 subset (E1-4 "Statement").

use std::ops::Range;

use super::lexer::Token;
use super::position::{Failure, outside_subset, unparsable};

/// A clause: its keyword and the tokens after it, as a range of token indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Clause {
    /// Its keyword.
    pub(super) keyword: ClauseKeyword,
    /// The tokens of its body.
    pub(super) tokens: Range<usize>,
}

/// The clauses found before the first structural failure, and the failures of the structure:
/// the first token that breaks it, and each clause without a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Scan {
    /// The clauses, in text order.
    pub(super) clauses: Vec<Clause>,
    /// The failures, in text order.
    pub(super) failures: Vec<Failure>,
}

/// The clause keywords, in the only order the subset allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ClauseKeyword {
    /// `SELECT`.
    Select,
    /// `FROM`.
    From,
    /// `WHERE`.
    Where,
    /// `GROUP BY`.
    GroupBy,
    /// `ORDER BY`.
    OrderBy,
    /// `LIMIT`.
    Limit,
}

impl ClauseKeyword {
    /// The keyword as written in messages.
    pub(super) fn name(self) -> &'static str {
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

/// Scans the statement made of `tokens`, one `SELECT` with an optional trailing `;`, whose text
/// a reader stops reading at byte `end`.
pub(super) fn scan(sql: &str, tokens: &[Token], end: usize) -> Scan {
    let count = match tokens.last() {
        Some(last) if last.is_symbol(';') => tokens.len() - 1,
        _ => tokens.len(),
    };
    let tokens = tokens.get(..count).unwrap_or_default();
    let (starts, stop, failure) = match clause_starts(sql, tokens, end) {
        Ok(starts) => (starts, tokens.len(), None),
        Err(broken) => (broken.starts, broken.stop, Some(broken.failure)),
    };
    let mut clauses = Vec::new();
    let mut failures = Vec::new();
    for (index, &(keyword, body_start)) in starts.iter().enumerate() {
        let body_end = starts.get(index + 1).map_or(stop, |&(next, next_start)| {
            next_start - keyword_length(next)
        });
        if body_start >= body_end {
            failures.push(unparsable(
                sql,
                next_offset(tokens, body_start, end),
                &format!("expected the body of {}", keyword.name()),
            ));
        }
        clauses.push(Clause {
            keyword,
            tokens: body_start..body_end.max(body_start),
        });
    }
    failures.extend(failure);
    failures.sort_by_key(|failure| failure.offset);
    Scan { clauses, failures }
}

/// The offset of the token at `index`, or `end` after the last token.
pub(super) fn next_offset(tokens: &[Token], index: usize, end: usize) -> usize {
    tokens.get(index).map_or(end, |token| token.start)
}

/// The clause keywords found while scanning, each with the index of the first token of its
/// body.
type Starts = Vec<(ClauseKeyword, usize)>;

/// The number of tokens of the clause keyword.
fn keyword_length(keyword: ClauseKeyword) -> usize {
    match keyword {
        ClauseKeyword::GroupBy | ClauseKeyword::OrderBy => 2,
        _ => 1,
    }
}

/// Each clause keyword of `tokens`, in order, with the index of the first token of its body;
/// on a structural failure, the keywords before it, the index of the failing token and the
/// failure.
fn clause_starts(sql: &str, tokens: &[Token], end: usize) -> Result<Starts, Box<Broken>> {
    check_first(sql, tokens, end).map_err(|failure| broken(Vec::new(), 0, failure))?;
    let mut starts = Vec::new();
    let mut depth = 0_usize;
    let mut index = 0;
    while index < tokens.len() {
        match structural_step(
            sql,
            tokens,
            index,
            depth,
            starts.last().map(|&(last, _)| last),
        ) {
            Ok(Step::Next(new_depth)) => {
                depth = new_depth;
                index += 1;
            }
            Ok(Step::Skip(length)) => index += length,
            Ok(Step::Clause(keyword)) => {
                index += keyword_length(keyword);
                starts.push((keyword, index));
            }
            Err(failure) => return Err(broken(starts, index, failure)),
        }
    }
    if depth > 0 {
        return Err(broken(
            starts,
            index,
            unparsable(sql, end, "a `(` is never closed"),
        ));
    }
    Ok(starts)
}

/// A structure that breaks: the clause keywords before the failure, the index of the token
/// where the scan stops, and the failure.
struct Broken {
    starts: Starts,
    stop: usize,
    failure: Failure,
}

fn broken(starts: Starts, stop: usize, failure: Failure) -> Box<Broken> {
    Box::new(Broken {
        starts,
        stop,
        failure,
    })
}

/// `EQRY-05` unless the statement starts with `SELECT`.
fn check_first(sql: &str, tokens: &[Token], end: usize) -> Result<(), Failure> {
    let Some(first) = tokens.first() else {
        return Err(unparsable(sql, end, "expected SELECT"));
    };
    if first.is_keyword(sql, "WITH") {
        return Err(outside_subset(sql, first.start, "WITH"));
    }
    if !first.is_keyword(sql, "SELECT") {
        return Err(unparsable(sql, first.start, "expected SELECT"));
    }
    Ok(())
}

/// What the scan does at one token.
enum Step {
    /// Go to the next token, at this parenthesis depth.
    Next(usize),
    /// Step over this many tokens.
    Skip(usize),
    /// A clause keyword starts here.
    Clause(ClauseKeyword),
}

/// The step at `tokens[index]`, at parenthesis `depth`, after the clause `last`.
fn structural_step(
    sql: &str,
    tokens: &[Token],
    index: usize,
    depth: usize,
    last: Option<ClauseKeyword>,
) -> Result<Step, Failure> {
    let Some(token) = tokens.get(index) else {
        return Ok(Step::Skip(1));
    };
    if token.is_symbol('(') {
        return Ok(Step::Next(depth + 1));
    }
    if token.is_symbol(')') {
        return depth
            .checked_sub(1)
            .map(Step::Next)
            .ok_or_else(|| unparsable(sql, token.start, "`)` closes no parenthesis"));
    }
    if token.is_symbol(';') {
        return Err(outside_subset(sql, token.start, "a second statement"));
    }
    if token.is_keyword(sql, "SELECT") && depth > 0 {
        return Err(outside_subset(sql, token.start, "a subquery"));
    }
    if depth > 0 {
        return Ok(Step::Next(depth));
    }
    if token.is_keyword(sql, "DISTINCT") {
        return distinct_length(sql, tokens, index).map(Step::Skip);
    }
    if let Some(keyword) = clause_keyword(sql, tokens, index)? {
        if last.is_some_and(|last| last >= keyword) {
            return Err(unparsable(
                sql,
                token.start,
                &format!("unexpected {}", keyword.name()),
            ));
        }
        return Ok(Step::Clause(keyword));
    }
    check_supported(sql, token)?;
    Ok(Step::Next(depth))
}

/// The clause keyword that starts at `tokens[index]`, if any.
fn clause_keyword(
    sql: &str,
    tokens: &[Token],
    index: usize,
) -> Result<Option<ClauseKeyword>, Failure> {
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
fn distinct_length(sql: &str, tokens: &[Token], index: usize) -> Result<usize, Failure> {
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
fn check_supported(sql: &str, token: &Token) -> Result<(), Failure> {
    match UNSUPPORTED_KEYWORDS
        .iter()
        .find(|(word, _)| token.is_keyword(sql, word))
    {
        Some((_, construct)) => Err(outside_subset(sql, token.start, construct)),
        None => Ok(()),
    }
}

/// The ranges of the items of `clause`, separated by commas at depth 0; the text is read up to
/// byte `end`.
///
/// # Errors
///
/// `EQRY-05` at a comma with no item before it, or after the clause when its last item is
/// empty.
pub(super) fn items(
    sql: &str,
    tokens: &[Token],
    clause: &Clause,
    end: usize,
) -> Result<Vec<Range<usize>>, Failure> {
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
        let offset = next_offset(tokens, start, end);
        return Err(unparsable(sql, offset, "expected an item"));
    }
    items.push(start..clause.tokens.end);
    Ok(items)
}
