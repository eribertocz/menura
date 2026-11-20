//! The text of one `SELECT` and of the whole statement, in the fixed format of E6-4.9:
//! keywords in upper case, one clause per line, two-space indentation, no trailing spaces.

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
            lines.push(format!("{indent}{INDENT}{item}{separator}"));
        }
        lines.push(format!("{indent}FROM {}", self.from));
        for clause in &self.clauses {
            lines.push(format!("{indent}{clause}"));
        }
        lines
    }
}

/// The statement `WITH` every named body, in order, then the final `SELECT` (E6-4.1, E6-4.4).
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
    lines.join("\n")
}
