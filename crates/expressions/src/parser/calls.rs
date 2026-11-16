//! Function calls: scalar functions, aggregates with `FILTER`, `AGG(metric)`,
//! `USER_ATTRIBUTE('name')`, and the detection of calls outside the subset and of window
//! functions (E0F-2.3, E0F-2.4).

use semantic_compiler_model::expression::{
    AggregateCall, DistinctAggregate, Expression, PercentileFunction, SimpleAggregate,
    SortDirection,
};

use super::{Parsed, Parser};
use crate::error::{ExpressionError, ExpressionErrorKind, Span};
use crate::lexer::{Symbol, TokenKind};
use crate::vocabulary::{
    accepts_argument_count, is_keyword, scalar_function, scalar_function_name, simple_aggregate,
    simple_aggregate_name,
};

impl Parser<'_> {
    /// A call `word(…)`; the next token is `word` and the one after it is `(`.
    pub(super) fn call(&mut self, word: &str) -> Parsed<Expression> {
        let start = self.next_span();
        if is_keyword(word, "AGG") {
            return self.metric_aggregate();
        }
        if is_keyword(word, "USER_ATTRIBUTE") {
            return self.user_attribute();
        }
        if let Some(function) = scalar_function(word) {
            self.position += 2;
            let arguments = self.expression_list()?;
            self.expect_symbol(Symbol::RightParenthesis)?;
            if !accepts_argument_count(function, arguments.len()) {
                return Err(ExpressionError::new(
                    ExpressionErrorKind::WrongArgumentCount {
                        function: scalar_function_name(function),
                        found: arguments.len(),
                    },
                    start.to(self.previous_span()),
                ));
            }
            return Ok(Expression::Function {
                function,
                arguments,
            });
        }
        let call = if let Some(function) = percentile_function(word) {
            self.percentile(function)?
        } else if let Some(function) = simple_aggregate(word) {
            self.aggregate(function, start)?
        } else {
            return Err(self.unsupported_call(start));
        };
        let filter = self.filter()?;
        if self.keyword_at(0, "OVER") {
            return Err(ExpressionError::new(
                ExpressionErrorKind::WindowFunction,
                start.to(self.next_span()),
            ));
        }
        Ok(Expression::Aggregate { call, filter })
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

    /// `FUNCTION(*)` (only `COUNT`), `FUNCTION(DISTINCT x)` (only `SUM` and `COUNT`) or
    /// `FUNCTION(x)`.
    fn aggregate(&mut self, function: SimpleAggregate, start: Span) -> Parsed<AggregateCall> {
        self.position += 2;
        if function == SimpleAggregate::Count && self.eat_symbol(Symbol::Star) {
            self.expect_symbol(Symbol::RightParenthesis)?;
            return Ok(AggregateCall::CountRows);
        }
        if self.keyword_at(0, "ALL") {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "ALL in an aggregate",
            )));
        }
        let distinct = if self.keyword_at(0, "DISTINCT") {
            let Some(distinct) = distinct_aggregate(function) else {
                return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                    "DISTINCT in an aggregate other than SUM and COUNT",
                )));
            };
            self.position += 1;
            Some(distinct)
        } else {
            None
        };
        let mut arguments = self.expression_list()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        let (Some(argument), true) = (arguments.pop(), arguments.is_empty()) else {
            return Err(ExpressionError::new(
                ExpressionErrorKind::WrongArgumentCount {
                    function: simple_aggregate_name(function),
                    found: arguments.len() + 1,
                },
                start.to(self.previous_span()),
            ));
        };
        let argument = Box::new(argument);
        Ok(match distinct {
            Some(function) => AggregateCall::Distinct { function, argument },
            None => AggregateCall::Simple { function, argument },
        })
    }

    /// `PERCENTILE_…(fraction) WITHIN GROUP (ORDER BY ordering [ASC | DESC])`.
    fn percentile(&mut self, function: PercentileFunction) -> Parsed<AggregateCall> {
        self.position += 2;
        let fraction = self.expression()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        self.expect_keyword("WITHIN")?;
        self.expect_keyword("GROUP")?;
        self.expect_symbol(Symbol::LeftParenthesis)?;
        self.expect_keyword("ORDER")?;
        self.expect_keyword("BY")?;
        let ordering = self.expression()?;
        let direction = if self.eat_keyword("DESC") {
            SortDirection::Descending
        } else {
            self.eat_keyword("ASC");
            SortDirection::Ascending
        };
        if self.keyword_at(0, "NULLS") {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "NULLS FIRST and NULLS LAST",
            )));
        }
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(AggregateCall::Percentile {
            function,
            fraction: Box::new(fraction),
            ordering: Box::new(ordering),
            direction,
        })
    }

    /// An optional `FILTER (WHERE condition)`.
    fn filter(&mut self) -> Parsed<Option<Box<Expression>>> {
        if !(self.keyword_at(0, "FILTER") && self.symbol_at(1, Symbol::LeftParenthesis)) {
            return Ok(None);
        }
        self.position += 2;
        self.expect_keyword("WHERE")?;
        let condition = self.expression()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(Some(Box::new(condition)))
    }

    /// The error for a call of a function outside the subset, whose name is the next token.
    ///
    /// The arguments are skipped without being parsed, since such functions may have their own
    /// syntax; a following `OVER` makes it a window function (E0F-2.4).
    fn unsupported_call(&mut self, start: Span) -> ExpressionError {
        let name = self.text[start.start..start.end].to_owned();
        let unsupported =
            ExpressionError::new(ExpressionErrorKind::UnsupportedFunction(name), start);
        self.position += 1;
        let mut depth = 0_usize;
        while let Some(token) = self.advance() {
            match token.kind {
                TokenKind::Symbol(Symbol::LeftParenthesis) => depth += 1,
                TokenKind::Symbol(Symbol::RightParenthesis) => depth = depth.saturating_sub(1),
                TokenKind::Invalid(_) => return unsupported,
                _ => {}
            }
            if depth == 0 {
                break;
            }
        }
        if depth == 0 && self.keyword_at(0, "OVER") {
            return ExpressionError::new(
                ExpressionErrorKind::WindowFunction,
                start.to(self.next_span()),
            );
        }
        unsupported
    }
}

fn percentile_function(word: &str) -> Option<PercentileFunction> {
    if is_keyword(word, "PERCENTILE_CONT") {
        Some(PercentileFunction::Continuous)
    } else if is_keyword(word, "PERCENTILE_DISC") {
        Some(PercentileFunction::Discrete)
    } else {
        None
    }
}

fn distinct_aggregate(function: SimpleAggregate) -> Option<DistinctAggregate> {
    match function {
        SimpleAggregate::Sum => Some(DistinctAggregate::Sum),
        SimpleAggregate::Count => Some(DistinctAggregate::Count),
        _ => None,
    }
}
