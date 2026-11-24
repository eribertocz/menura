//! The text of one `SELECT` and of the whole statement, in the fixed format of E6-4.9:
//! keywords in upper case, one clause per line, two-space indentation, no trailing spaces.

use crate::settings::SETTINGS;

/// One `SELECT`: its items, its `FROM` and its further clauses, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectBody {
    /// The select items, in order.
    pub(crate) items: Vec<String>,
    /// The text after `FROM`.
    pub(crate) from: String,
    /// The clauses after `FROM`, in order, each on its own line (`WHERE …`, `LIMIT …`).
    pub(crate) clauses: Vec<String>,
}

const INDENT: &str = "  ";

impl SelectBody {
    /// The lines of the body, indented by `depth` levels.
    fn lines(&self, depth: usize) -> Vec<String> {
        let indent = INDENT.repeat(depth);
        let mut lines = vec![format!("{indent}SELECT")];
        let last = self.items.len().saturating_sub(1);
        for (index, item) in self.items.iter().enumerate() {
            let separator = if index == last { "" } else { "," };
            let item = named_item(item);
            lines.push(format!("{indent}{INDENT}{item}{separator}"));
        }
        match self.from.rsplit_once('\n') {
            Some((text, closing)) => {
                lines.push(format!("{indent}FROM {text}"));
                lines.push(format!("{indent}{closing}"));
            }
            None => lines.push(format!("{indent}FROM {}", self.from)),
        }
        for clause in &self.clauses {
            lines.push(format!("{indent}{clause}"));
        }
        lines
    }
}

/// `item`, named explicitly when it is a qualified column `"alias"."column"` and nothing else:
/// `"alias"."column" AS "column"`. `ClickHouse` may name such an item `alias.column` across a
/// join, so a step that reads it by `"column"` would not find it; an explicit name keeps the
/// output names of every step independent of that naming (E6-7.1).
fn named_item(item: &str) -> String {
    match qualified_column(item) {
        Some(column) => format!("{item} AS {column}"),
        None => item.to_owned(),
    }
}

/// The quoted column of `item` when `item` is exactly `"alias"."column"`.
fn qualified_column(item: &str) -> Option<&str> {
    let (_, rest) = quoted_prefix(item)?;
    let column = rest.strip_prefix('.')?;
    let (column_length, rest) = quoted_prefix(column)?;
    rest.is_empty().then_some(&column[..column_length])
}

/// The length of the quoted identifier that starts `text`, every inner double quote doubled
/// (E6-7.1), and the text after it.
fn quoted_prefix(text: &str) -> Option<(usize, &str)> {
    let body = text.strip_prefix('"')?;
    let mut characters = body.char_indices().peekable();
    while let Some((index, character)) = characters.next() {
        if character != '"' {
            continue;
        }
        if characters.next_if(|(_, next)| *next == '"').is_none() {
            let length = index + 2;
            return Some((length, &text[length..]));
        }
    }
    None
}

/// `first`, then `UNION ALL` and each of `others`, one clause per line (E6-4.9).
pub(crate) fn union_all(first: SelectBody, others: &[SelectBody]) -> SelectBody {
    let mut body = first;
    for other in others {
        body.clauses.push("UNION ALL".to_owned());
        body.clauses.extend(other.lines(0));
    }
    body
}

/// The statement `WITH` every named body, in order, then the final `SELECT` (E6-4.1, E6-4.4),
/// then the `SETTINGS` clause, one setting per line (E6-7.8, E6-4.9).
pub(crate) fn statement(steps: &[(String, SelectBody)], last: &SelectBody) -> String {
    let mut lines = vec!["WITH".to_owned()];
    let count = steps.len();
    for (index, (name, body)) in steps.iter().enumerate() {
        lines.push(format!("{INDENT}{name} AS ("));
        lines.extend(body.lines(2));
        let separator = if index + 1 == count { "" } else { "," };
        lines.push(format!("{INDENT}){separator}"));
    }
    lines.extend(last.lines(0));
    lines.push("SETTINGS".to_owned());
    let final_setting = SETTINGS.len().saturating_sub(1);
    for (index, setting) in SETTINGS.iter().enumerate() {
        let separator = if index == final_setting { "" } else { "," };
        lines.push(format!("{INDENT}{setting}{separator}"));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::{SelectBody, named_item};

    #[test]
    fn e6_7_1_a_qualified_column_is_selected_under_its_own_name() {
        assert_eq!(
            named_item("\"menura_fill\".\"entity_1\""),
            "\"menura_fill\".\"entity_1\" AS \"entity_1\""
        );
        assert_eq!(
            named_item("\"a\"\"b\".\"c.\"\"d\""),
            "\"a\"\"b\".\"c.\"\"d\" AS \"c.\"\"d\""
        );
    }

    #[test]
    fn e6_7_1_other_items_keep_their_text() {
        for item in [
            "\"entity_1\"",
            "\"menura_fill\".\"entity_1\" AS \"entity_1\"",
            "\"menura_value\".\"x\" = \"menura_expected\".\"y\"",
            "NULL AS \"entity_1\"",
            "max(\"a\".\"b\")",
            "\"a\".\"b\".\"c\"",
        ] {
            assert_eq!(named_item(item), item);
        }
    }

    #[test]
    fn e6_7_1_a_select_list_names_its_qualified_columns() {
        let body = SelectBody {
            items: vec![
                "\"menura_fill\".\"entity_1\"".to_owned(),
                "\"menura_missing\".\"menura.period\"".to_owned(),
            ],
            from: "\"menura_10_missing\" AS \"menura_missing\"".to_owned(),
            clauses: Vec::new(),
        };
        assert_eq!(
            body.lines(0),
            vec![
                "SELECT",
                "  \"menura_fill\".\"entity_1\" AS \"entity_1\",",
                "  \"menura_missing\".\"menura.period\" AS \"menura.period\"",
                "FROM \"menura_10_missing\" AS \"menura_missing\"",
            ]
        );
    }
}
