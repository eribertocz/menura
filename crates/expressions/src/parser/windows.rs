//! The window clause `OVER (…)` of the grammar (`overClause` of `OssieExpression.g4`).
//!
//! Window functions are not part of version 1 (E0F-2.4), so the clause is read only to let the
//! grammar decide first (E0F-9.1): a clause it rejects is `E0F-01`, and one it accepts makes the
//! call a window function. Nothing of the clause is kept.

use super::{Parsed, Parser};
use crate::error::{ExpressionError, ExpressionErrorKind, Span};
use crate::lexer::Symbol;

impl Parser<'_> {
    /// The error for a call starting at `start` whose next token is `OVER`: the clause's syntax
    /// error if it has one, otherwise [`ExpressionErrorKind::WindowFunction`] over the call and
    /// its clause.
    pub(super) fn window_function(&mut self, start: Span) -> ExpressionError {
        match self.over_clause() {
            Ok(()) => ExpressionError::new(
                ExpressionErrorKind::WindowFunction,
                start.to(self.previous_span()),
            ),
            Err(error) => error,
        }
    }

    /// `OVER ( [PARTITION BY expr, …] [ORDER BY sortItem, …] [frameClause] )`.
    fn over_clause(&mut self) -> Parsed<()> {
        self.expect_keyword("OVER")?;
        self.expect_symbol(Symbol::LeftParenthesis)?;
        if self.eat_keyword("PARTITION") {
            self.expect_keyword("BY")?;
            self.expression_list()?;
        }
        if self.keyword_at(0, "ORDER") {
            self.sort_items()?;
        }
        if self.eat_keyword("ROWS") || self.eat_keyword("RANGE") {
            self.frame_extent()?;
        }
        self.expect_symbol(Symbol::RightParenthesis)
    }

    /// After `ROWS` or `RANGE`: `BETWEEN frameBound AND frameBound`, or one `frameBound`.
    fn frame_extent(&mut self) -> Parsed<()> {
        if self.eat_keyword("BETWEEN") {
            self.frame_bound()?;
            self.expect_keyword("AND")?;
        }
        self.frame_bound()
    }

    /// `UNBOUNDED PRECEDING`, `UNBOUNDED FOLLOWING`, `CURRENT ROW`, or `expr PRECEDING` /
    /// `expr FOLLOWING`.
    ///
    /// The offset is read at the level of `+` and `-`, so the `AND` of `BETWEEN … AND …` is
    /// never taken as part of it.
    fn frame_bound(&mut self) -> Parsed<()> {
        if self.eat_keyword("CURRENT") {
            return self.expect_keyword("ROW");
        }
        if !self.eat_keyword("UNBOUNDED") {
            self.additive()?;
        }
        if self.eat_keyword("PRECEDING") || self.eat_keyword("FOLLOWING") {
            Ok(())
        } else {
            Err(self.unexpected("PRECEDING or FOLLOWING"))
        }
    }
}
