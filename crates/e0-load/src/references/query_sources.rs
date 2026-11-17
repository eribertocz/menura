//! `E0D-08`: a `source` that is a query is a single read-only `SELECT` (E0D-3.3, E6-4.3).
//!
//! The check is lexical. It guards the model text; the executor's read-only identity remains the
//! protection at run time.

use super::diagnostic::ReferenceCode;
use super::finding::Finding;
use super::scope::Scope;

/// The words that make a query source fail `E0D-08` (E0D-3.3), compared without case.
const FORBIDDEN_WORDS: [&str; 29] = [
    "INSERT",
    "UPDATE",
    "DELETE",
    "MERGE",
    "UPSERT",
    "REPLACE",
    "CREATE",
    "DROP",
    "ALTER",
    "TRUNCATE",
    "COPY",
    "ATTACH",
    "DETACH",
    "INSTALL",
    "LOAD",
    "PRAGMA",
    "SET",
    "RESET",
    "CALL",
    "EXPORT",
    "IMPORT",
    "INTO",
    "VACUUM",
    "CHECKPOINT",
    "GRANT",
    "REVOKE",
    "BEGIN",
    "COMMIT",
    "ROLLBACK",
];

/// Every `E0D-08` finding of the model.
pub(crate) fn query_source_findings(scope: &Scope<'_, '_>) -> Vec<Finding> {
    scope
        .model
        .datasets
        .iter()
        .filter_map(|dataset| dataset.source.as_ref())
        .filter(|source| is_query(source.text))
        .filter_map(|source| {
            let problem = problem(source.text)?;
            Some(Finding::rejecting(
                ReferenceCode::UnsafeQuerySource,
                source,
                format!("the query source is not a single read-only SELECT: {problem}"),
            ))
        })
        .collect()
}

/// Whether `source` is a query (E6-4.3): after leading whitespace, it starts with the word
/// `SELECT` or `WITH`, in any case, followed by a character that cannot continue an unquoted
/// identifier or by the end of the text.
pub(crate) fn is_query(source: &str) -> bool {
    let start = source.trim_start();
    ["SELECT", "WITH"].iter().any(|keyword| {
        let ends_word = start
            .get(keyword.len()..)
            .is_some_and(|rest| !rest.starts_with(continues_identifier));
        ends_word
            && start
                .get(..keyword.len())
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword))
    })
}

/// Whether `character` can continue an unquoted identifier: a letter, a digit, `_` or `$`
/// (E6-4.3).
fn continues_identifier(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

/// Why the query `text` fails E0D-3.3, or `None` when it passes.
fn problem(text: &str) -> Option<String> {
    let tokens = match tokens(text) {
        Ok(tokens) => tokens,
        Err(unterminated) => return Some(format!("{unterminated} is not terminated")),
    };
    if let Some(position) = tokens.iter().position(|token| *token == Token::Semicolon)
        && position + 1 != tokens.len()
    {
        return Some("`;` separates statements".to_owned());
    }
    let mut words = tokens.iter().filter_map(|token| match token {
        Token::Word(word) => Some(*word),
        Token::Semicolon => None,
    });
    match words.next() {
        Some(first)
            if first.eq_ignore_ascii_case("SELECT") || first.eq_ignore_ascii_case("WITH") => {}
        _ => return Some("its first keyword is not SELECT or WITH".to_owned()),
    }
    words
        .find(|word| {
            FORBIDDEN_WORDS
                .iter()
                .any(|forbidden| word.eq_ignore_ascii_case(forbidden))
        })
        .map(|word| format!("it contains the keyword `{word}`"))
}

/// A token that the check reads; everything else (literals, quoted identifiers, comments,
/// operators, numbers) is skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Token<'text> {
    /// A letter or `_`, then the characters that can continue an unquoted identifier.
    Word(&'text str),
    /// `;`.
    Semicolon,
}

/// The words and semicolons of `text` outside string literals, quoted identifiers and comments.
///
/// # Errors
///
/// What is not terminated: a string literal, a quoted identifier or a block comment.
fn tokens(text: &str) -> Result<Vec<Token<'_>>, &'static str> {
    let mut tokens = Vec::new();
    let mut rest = text;
    while let Some(character) = rest.chars().next() {
        let skipped = match character {
            '\'' => quoted(rest, '\'').ok_or("a string literal")?,
            '"' => quoted(rest, '"').ok_or("a quoted identifier")?,
            '-' if rest.starts_with("--") => rest.find('\n').unwrap_or(rest.len()),
            '/' if rest.starts_with("/*") => rest
                .get(2..)
                .and_then(|body| body.find("*/"))
                .map(|end| end + 4)
                .ok_or("a block comment")?,
            ';' => {
                tokens.push(Token::Semicolon);
                1
            }
            _ if character.is_alphabetic() || character == '_' => {
                let length = rest
                    .find(|next: char| !continues_identifier(next))
                    .unwrap_or(rest.len());
                tokens.push(Token::Word(&rest[..length]));
                length
            }
            _ if character.is_numeric() => rest
                .find(|next: char| !continues_identifier(next))
                .unwrap_or(rest.len()),
            _ => character.len_utf8(),
        };
        rest = &rest[skipped..];
    }
    Ok(tokens)
}

/// The length of the literal or identifier quoted with `quote` at the start of `text`, where a
/// doubled quote stands for one; `None` when it is not terminated.
fn quoted(text: &str, quote: char) -> Option<usize> {
    let mut position = quote.len_utf8();
    loop {
        let end = position + text.get(position..)?.find(quote)?;
        let after = end + quote.len_utf8();
        if text.get(after..)?.starts_with(quote) {
            position = after + quote.len_utf8();
        } else {
            return Some(after);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{is_query, problem};

    fn passes(text: &str) -> bool {
        problem(text).is_none()
    }

    #[test]
    fn e6_4_3_a_source_is_a_query_when_it_starts_with_the_word_select_or_with() {
        assert!(is_query("  select 1"));
        assert!(is_query("SELECT"));
        assert!(is_query("select*from t"));
        assert!(is_query("WITH\nt AS (SELECT 1) SELECT * FROM t"));
        assert!(is_query("SELECT(1)"));
        assert!(!is_query("withdrawals.main.entries"));
        assert!(!is_query("selected.main.items"));
        assert!(!is_query("SELECT_ALL.main.items"));
        assert!(!is_query("with$.main.t"));
        assert!(!is_query("selecté.main.t"));
        assert!(!is_query("with1.main.t"));
        assert!(is_query("WITH t AS (SELECT 1) SELECT * FROM t"));
        assert!(!is_query("shop.main.orders"));
        assert!(!is_query("/* c */ SELECT 1"));
        assert!(!is_query("é"));
    }

    #[test]
    fn e0d_3_3_a_single_read_only_select_passes() {
        assert!(passes("SELECT order_id FROM shop.main.orders"));
        assert!(passes("SELECT order_id FROM shop.main.orders;"));
        assert!(passes("SELECT order_id FROM shop.main.orders; -- end\n"));
        assert!(passes("with t as (select 1 as x) select x from t"));
        assert!(passes(
            "SELECT 'DROP; it' AS \"INSERT\"\"x;\", 2e5 -- DELETE\nFROM t /* UPDATE; */"
        ));
        assert!(passes("SELECT updated_at, inserts, set$1 FROM t"));
    }

    #[test]
    fn e0d_3_3_e0d_08_statements_writes_and_malformed_text_fail() {
        assert!(!passes("SELECT 1; SELECT 2"));
        assert!(!passes("SELECT 1;;"));
        assert!(!passes(
            "WITH d AS (DELETE FROM t RETURNING *) SELECT * FROM d"
        ));
        assert!(!passes("SELECT * INTO copy FROM t"));
        assert!(!passes("select 1; drop table t"));
        assert!(!passes("SELECT 'open"));
        assert!(!passes("SELECT \"open"));
        assert!(!passes("SELECT 1 /* open"));
        assert_eq!(
            problem("SELECT x FROM t WHERE y IN (SELECT 1) UNION SELECT pragma_version()"),
            None
        );
        assert_eq!(
            problem("SELECT x FROM t; SET a = 1").as_deref(),
            Some("`;` separates statements")
        );
        assert_eq!(
            problem("SELECT * FROM read_csv('x') Merge").as_deref(),
            Some("it contains the keyword `Merge`")
        );
    }
}
