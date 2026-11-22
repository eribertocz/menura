//! Whether a Layer 2 query orders its result: upstream `compliance/query/README.md` ("Comparing
//! results") compares the row order only when `query.ossie.sql` has `ORDER BY`.

/// Whether the outermost query of `sql` has `ORDER BY`. Comments, quoted text and everything
/// inside parentheses (subqueries, window definitions) are skipped.
pub(crate) fn has_outer_order_by(sql: &str) -> bool {
    let words = outer_words(sql);
    words
        .windows(2)
        .any(|pair| pair[0].eq_ignore_ascii_case("ORDER") && pair[1].eq_ignore_ascii_case("BY"))
}

/// The words of `sql` at parenthesis depth zero, outside comments and quoted text.
fn outer_words(sql: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut depth: usize = 0;
    let mut characters = sql.chars().peekable();
    while let Some(character) = characters.next() {
        let separates = !(character.is_ascii_alphanumeric() || character == '_');
        if separates && !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
        match character {
            '-' if characters.peek() == Some(&'-') => skip_line(&mut characters),
            '/' if characters.peek() == Some(&'*') => skip_block_comment(&mut characters),
            '\'' | '"' => skip_quoted(&mut characters, character),
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if !separates && depth == 0 => word.push(character),
            _ => {}
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
}

fn skip_line(characters: &mut impl Iterator<Item = char>) {
    for character in characters.by_ref() {
        if character == '\n' {
            return;
        }
    }
}

fn skip_block_comment(characters: &mut std::iter::Peekable<impl Iterator<Item = char>>) {
    characters.next();
    while let Some(character) = characters.next() {
        if character == '*' && characters.peek() == Some(&'/') {
            characters.next();
            return;
        }
    }
}

/// Skips to the end of a quoted text opened by `quote`; a doubled quote stays inside it.
fn skip_quoted(characters: &mut std::iter::Peekable<impl Iterator<Item = char>>, quote: char) {
    while let Some(character) = characters.next() {
        if character == quote {
            if characters.peek() == Some(&quote) {
                characters.next();
            } else {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cs_4_6_an_outer_order_by_orders_the_result() {
        assert!(has_outer_order_by(
            "SELECT deptno FROM scott_model.emp GROUP BY deptno\norder  by deptno;"
        ));
    }

    #[test]
    fn cs_4_6_a_query_without_order_by_is_unordered() {
        assert!(!has_outer_order_by(
            "SELECT AGG(sum_sal) FROM scott_model.emp;"
        ));
    }

    #[test]
    fn cs_4_6_order_by_in_comments_text_or_parentheses_does_not_count() {
        let sql = "-- ORDER BY in a comment\n/* ORDER BY */ SELECT 'ORDER BY' AS x, \
                   SUM(a) OVER (ORDER BY b) FROM (SELECT * FROM t ORDER BY a) AS s;";
        assert!(!has_outer_order_by(sql));
    }
}
