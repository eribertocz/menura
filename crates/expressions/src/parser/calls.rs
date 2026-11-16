//! Function calls: dispatch by name, the plain scalar functions of E0F-10, `AGG(metric)`,
//! `USER_ATTRIBUTE('name')`, and the detection of calls outside the subset and of window
//! functions (E0F-2.4, E0F-2.6, E0F-9.4).

use semantic_compiler_model::expression::Expression;

use super::{Parsed, Parser};
use crate::error::{ExpressionError, ExpressionErrorKind, Span};
use crate::lexer::{Symbol, TokenKind};
use crate::vocabulary::{Arity, is_keyword, scalar_function, scalar_function_name};

/// The arguments of a call as the grammar reads them, before E0F-9.4 checks the form.
pub(super) struct CallArguments {
    /// `(*)`.
    pub(super) star: bool,
    /// Where `DISTINCT` was written, if it was.
    pub(super) distinct: Option<Span>,
    /// The arguments, in order.
    pub(super) arguments: Vec<Expression>,
}

impl Parser<'_> {
    /// A call `word(…)`; the next token is `word` and the one after it is `(`.
    pub(super) fn call(&mut self, word: &str) -> Parsed<Expression> {
        let start = self.next_span();
        let expression = if is_keyword(word, "AGG") {
            self.metric_aggregate()?
        } else if is_keyword(word, "USER_ATTRIBUTE") {
            self.user_attribute()?
        } else if let Some(expression) = self.date_call(word, start)? {
            expression
        } else if let Some((function, arity)) = scalar_function(word) {
            self.position += 2;
            let arguments = self.plain_arguments()?;
            check_arity(
                scalar_function_name(function),
                arity,
                arguments.len(),
                start.to(self.previous_span()),
            )?;
            Expression::Function {
                function,
                arguments,
            }
        } else if let Some(aggregate) = self.aggregate_call(word, start)? {
            return Ok(aggregate);
        } else {
            return Err(self.unsupported_call(start));
        };
        self.reject_aggregate_clauses(start)?;
        Ok(expression)
    }

    /// The arguments after `(`, through the closing `)`: `*`, or an optional `DISTINCT` and
    /// zero or more expressions.
    pub(super) fn call_arguments(&mut self) -> Parsed<CallArguments> {
        let mut arguments = CallArguments {
            star: false,
            distinct: None,
            arguments: Vec::new(),
        };
        if self.symbol_at(0, Symbol::Star) && self.symbol_at(1, Symbol::RightParenthesis) {
            self.position += 2;
            arguments.star = true;
            return Ok(arguments);
        }
        if self.keyword_at(0, "DISTINCT") {
            arguments.distinct = Some(self.next_span());
            self.position += 1;
        }
        if !(arguments.distinct.is_none() && self.symbol_at(0, Symbol::RightParenthesis)) {
            arguments.arguments = self.expression_list()?;
        }
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(arguments)
    }

    /// The arguments of a function that takes neither `*` nor `DISTINCT` (E0F-9.4).
    pub(super) fn plain_arguments(&mut self) -> Parsed<Vec<Expression>> {
        let start = self.next_span();
        let arguments = self.call_arguments()?;
        if arguments.star {
            return Err(ExpressionError::new(
                ExpressionErrorKind::UnsupportedConstruct("* outside COUNT(*)"),
                start,
            ));
        }
        if let Some(span) = arguments.distinct {
            return Err(ExpressionError::new(
                ExpressionErrorKind::UnsupportedConstruct("DISTINCT outside SUM and COUNT"),
                span,
            ));
        }
        Ok(arguments.arguments)
    }

    /// After a call that is not an aggregate: `OVER` makes it a window function (E0F-2.4), and
    /// `FILTER` or `WITHIN GROUP` is outside the subset (E0F-9.4).
    pub(super) fn reject_aggregate_clauses(&self, start: Span) -> Parsed<()> {
        if self.keyword_at(0, "OVER") {
            return Err(ExpressionError::new(
                ExpressionErrorKind::WindowFunction,
                start.to(self.next_span()),
            ));
        }
        let clause = (self.keyword_at(0, "FILTER") && self.symbol_at(1, Symbol::LeftParenthesis))
            || (self.keyword_at(0, "WITHIN") && self.keyword_at(1, "GROUP"));
        if clause {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "FILTER and WITHIN GROUP on a function that is not an aggregate of E0F-10.5",
            )));
        }
        Ok(())
    }

    /// `AGG(metric)`.
    fn metric_aggregate(&mut self) -> Parsed<Expression> {
        self.position += 2;
        let reference = self.reference()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(Expression::MetricAggregate(reference))
    }

    /// `USER_ATTRIBUTE('name')`: the argument must be a text literal.
    fn user_attribute(&mut self) -> Parsed<Expression> {
        self.position += 2;
        let Some(TokenKind::Text(name)) = self.peek_kind_at(0).cloned() else {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "USER_ATTRIBUTE with an argument that is not a text literal",
            )));
        };
        self.position += 1;
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(Expression::UserAttribute(name))
    }

    /// The error for a call of a function outside the subset, whose name is the next token.
    ///
    /// The call is read with the grammar's generic call form first (E0F-9.1), so a text the
    /// grammar rejects stays `E0F-01`: the arguments (with `*`, `DISTINCT`, or the `x AS type`
    /// of `TRY_CAST`), then an optional `WITHIN GROUP (ORDER BY …)` and `FILTER (WHERE …)`. An
    /// `OVER` after them makes it a window function (E0F-2.4); otherwise it is `E0F-02`.
    fn unsupported_call(&mut self, start: Span) -> ExpressionError {
        match self.generic_call(start) {
            Ok(()) if self.keyword_at(0, "OVER") => ExpressionError::new(
                ExpressionErrorKind::WindowFunction,
                start.to(self.next_span()),
            ),
            Ok(()) => {
                let name = self.text[start.start..start.end].to_owned();
                ExpressionError::new(ExpressionErrorKind::UnsupportedFunction(name), start)
            }
            Err(error) => error,
        }
    }

    /// Reads a call in the grammar's generic form, discarding it.
    fn generic_call(&mut self, start: Span) -> Parsed<()> {
        let is_try_cast = self.text[start.start..start.end].eq_ignore_ascii_case("TRY_CAST");
        self.position += 2;
        if is_try_cast {
            self.expression()?;
            self.expect_keyword("AS")?;
            self.type_name()?;
            self.expect_symbol(Symbol::RightParenthesis)?;
        } else {
            self.call_arguments()?;
        }
        if self.keyword_at(0, "WITHIN") && self.keyword_at(1, "GROUP") {
            self.position += 2;
            self.expect_symbol(Symbol::LeftParenthesis)?;
            self.sort_items()?;
            self.expect_symbol(Symbol::RightParenthesis)?;
        }
        self.filter()?;
        Ok(())
    }

    /// A type name with optional parameters, as the grammar reads it: a word, then an optional
    /// `(` list of integers `)`.
    fn type_name(&mut self) -> Parsed<()> {
        if !matches!(self.peek_kind_at(0), Some(TokenKind::Word(_))) {
            return Err(self.unexpected("a type"));
        }
        self.position += 1;
        if self.eat_symbol(Symbol::LeftParenthesis) {
            self.expression_list()?;
            self.expect_symbol(Symbol::RightParenthesis)?;
        }
        Ok(())
    }
}

/// `E0F-02` when a function of E0F-10 gets a number of arguments it does not take.
pub(super) fn check_arity(
    function: &'static str,
    arity: Arity,
    found: usize,
    span: Span,
) -> Parsed<()> {
    if arity.accepts(found) {
        Ok(())
    } else {
        Err(ExpressionError::new(
            ExpressionErrorKind::WrongArgumentCount { function, found },
            span,
        ))
    }
}

/// The arguments of a function that takes exactly `N` of them; `E0F-02` otherwise.
pub(super) fn exactly<const N: usize>(
    function: &'static str,
    arguments: Vec<Expression>,
    span: Span,
) -> Parsed<[Expression; N]> {
    let found = arguments.len();
    <[Expression; N]>::try_from(arguments).map_err(|_| {
        ExpressionError::new(
            ExpressionErrorKind::WrongArgumentCount { function, found },
            span,
        )
    })
}
