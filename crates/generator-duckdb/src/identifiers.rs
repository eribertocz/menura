//! Identifiers and dataset sources (E6-4.3, E6-5.1).

/// `name` as a `DuckDB` quoted identifier: between double quotes, with every double quote
/// doubled (E6-5.1, PLN-2.2).
pub(crate) fn quote(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// What a dataset `source` is (E6-4.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source<'source> {
    /// A query, emitted as a parenthesized subquery.
    Query(&'source str),
    /// A relation name, as the parts of its qualified name.
    Relation(Vec<String>),
}

const QUERY_KEYWORDS: [&str; 2] = ["SELECT", "WITH"];

/// What `source` is, or `None` when it is neither a query nor a qualified name.
pub(crate) fn classify_source(source: &str) -> Option<Source<'_>> {
    if is_query(source) {
        return Some(Source::Query(source.trim()));
    }
    relation_parts(source.trim()).map(Source::Relation)
}

/// `FROM` text of `source`: the subquery of a query, or the quoted parts of a relation.
pub(crate) fn source_sql(source: &Source<'_>) -> String {
    match source {
        Source::Query(text) => format!("({text})"),
        Source::Relation(parts) => parts
            .iter()
            .map(|part| quote(part))
            .collect::<Vec<_>>()
            .join("."),
    }
}

/// Whether `source`, after leading whitespace, starts with `SELECT` or `WITH` followed by a
/// character that cannot continue an unquoted identifier, or by the end of the text.
fn is_query(source: &str) -> bool {
    let text = source.trim_start();
    QUERY_KEYWORDS.iter().any(|keyword| {
        let Some(start) = text.get(..keyword.len()) else {
            return false;
        };
        let rest = &text[keyword.len()..];
        start.eq_ignore_ascii_case(keyword)
            && rest
                .chars()
                .next()
                .is_none_or(|next| !continues_identifier(next))
    })
}

fn continues_identifier(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

/// The parts of the qualified name `text`: separated by `.`, each unquoted or between double
/// quotes (where `""` is one quote and a dot is part of the name).
fn relation_parts(text: &str) -> Option<Vec<String>> {
    let mut parts = Vec::new();
    let mut characters = text.chars().peekable();
    loop {
        let part = if characters.peek() == Some(&'"') {
            characters.next();
            quoted_part(&mut characters)?
        } else {
            let part: String = characters
                .clone()
                .take_while(|character| *character != '.')
                .collect();
            for _ in part.chars() {
                characters.next();
            }
            part
        };
        if part.is_empty() {
            return None;
        }
        parts.push(part);
        match characters.next() {
            None => return Some(parts),
            Some('.') => {}
            Some(_) => return None,
        }
    }
}

/// The content of a quoted part, after its opening quote, up to and without its closing quote.
fn quoted_part(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Option<String> {
    let mut part = String::new();
    loop {
        match characters.next()? {
            '"' if characters.peek() == Some(&'"') => {
                characters.next();
                part.push('"');
            }
            '"' => return Some(part),
            character => part.push(character),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Source, classify_source, quote};

    fn relation(parts: &[&str]) -> Source<'static> {
        Source::Relation(parts.iter().map(|part| (*part).to_owned()).collect())
    }

    #[test]
    fn e6_5_1_quotes_are_doubled() {
        assert_eq!(quote("a\"b"), "\"a\"\"b\"");
    }

    #[test]
    fn e6_4_3_sources_are_classified() {
        assert_eq!(
            classify_source("main.orders"),
            Some(relation(&["main", "orders"]))
        );
        assert_eq!(
            classify_source(r#""my.db"."Main".x"#),
            Some(relation(&["my.db", "Main", "x"]))
        );
        assert_eq!(
            classify_source("withdrawals.main.entries"),
            Some(relation(&["withdrawals", "main", "entries"]))
        );
        assert_eq!(classify_source("with_x"), Some(relation(&["with_x"])));
        assert_eq!(
            classify_source("  SELECT 1 "),
            Some(Source::Query("SELECT 1"))
        );
        assert_eq!(classify_source("with"), Some(Source::Query("with")));
        assert_eq!(classify_source("a..b"), None);
        assert_eq!(classify_source("\"open"), None);
        assert_eq!(classify_source("\"a\"b"), None);
    }
}
