//! Aggregate calls of E0F-10.5, with their `FILTER (WHERE …)`, and the call forms E0F-9.4
//! allows on them.

use semantic_compiler_model::expression::{
    AggregateCall, DistinctAggregate, Expression, PercentileFunction, SimpleAggregate,
    SortDirection,
};

use super::calls::{CallArguments, exactly};
use super::{Parsed, Parser};
use crate::error::{ExpressionError, ExpressionErrorKind, Span};
use crate::lexer::Symbol;
use crate::vocabulary::{is_keyword, simple_aggregate, simple_aggregate_name};

impl Parser<'_> {
    /// An aggregate call named `word`, or `None` when `word` names no aggregate.
    pub(super) fn aggregate_call(&mut self, word: &str, start: Span) -> Parsed<Option<Expression>> {
        let call = if let Some(function) = percentile_function(word) {
            self.position += 2;
            self.percentile(function, start)?
        } else if let Some(function) = simple_aggregate(word) {
            self.position += 2;
            let arguments = self.call_arguments()?;
            self.simple_aggregate(function, arguments, start)?
        } else {
            return Ok(None);
        };
        if self.keyword_at(0, "WITHIN") && self.keyword_at(1, "GROUP") {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "WITHIN GROUP outside PERCENTILE_CONT and PERCENTILE_DISC",
            )));
        }
        let filter = self.filter()?;
        if self.keyword_at(0, "OVER") {
            return Err(ExpressionError::new(
                ExpressionErrorKind::WindowFunction,
                start.to(self.next_span()),
            ));
        }
        Ok(Some(Expression::Aggregate { call, filter }))
    }

    /// `COUNT(*)`, `SUM(DISTINCT x)`, `COUNT(DISTINCT x)` or `FUNCTION(x)` (E0F-9.4).
    fn simple_aggregate(
        &self,
        function: SimpleAggregate,
        arguments: CallArguments,
        start: Span,
    ) -> Parsed<AggregateCall> {
        let span = start.to(self.previous_span());
        if arguments.star {
            if function == SimpleAggregate::Count {
                return Ok(AggregateCall::CountRows);
            }
            return Err(ExpressionError::new(
                ExpressionErrorKind::UnsupportedConstruct("* outside COUNT(*)"),
                span,
            ));
        }
        let [argument] = exactly(simple_aggregate_name(function), arguments.arguments, span)?;
        let argument = Box::new(argument);
        let Some(distinct_span) = arguments.distinct else {
            return Ok(AggregateCall::Simple { function, argument });
        };
        let function = distinct_aggregate(function).ok_or_else(|| {
            ExpressionError::new(
                ExpressionErrorKind::UnsupportedConstruct("DISTINCT outside SUM and COUNT"),
                distinct_span,
            )
        })?;
        Ok(AggregateCall::Distinct { function, argument })
    }

    /// `PERCENTILE_…(fraction) WITHIN GROUP (ORDER BY ordering [ASC | DESC] [NULLS FIRST |
    /// NULLS LAST])`, after its `(`. `NULLS …` has no effect (E0F-10.5).
    fn percentile(&mut self, function: PercentileFunction, start: Span) -> Parsed<AggregateCall> {
        let name = match function {
            PercentileFunction::Continuous => "PERCENTILE_CONT",
            PercentileFunction::Discrete => "PERCENTILE_DISC",
        };
        let arguments = self.plain_arguments()?;
        let [fraction] = exactly(name, arguments, start.to(self.previous_span()))?;
        if !(self.keyword_at(0, "WITHIN") && self.keyword_at(1, "GROUP")) {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "PERCENTILE_CONT and PERCENTILE_DISC without WITHIN GROUP",
            )));
        }
        self.position += 2;
        self.expect_symbol(Symbol::LeftParenthesis)?;
        let sort_start = self.next_span();
        let mut items = self.sort_items()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        let (Some((ordering, direction)), true) = (items.pop(), items.is_empty()) else {
            return Err(ExpressionError::new(
                ExpressionErrorKind::UnsupportedConstruct(
                    "WITHIN GROUP with more than one sort item",
                ),
                sort_start.to(self.previous_span()),
            ));
        };
        Ok(AggregateCall::Percentile {
            function,
            fraction: Box::new(fraction),
            ordering: Box::new(ordering),
            direction,
        })
    }

    /// `ORDER BY item [, item …]`, each `expression [ASC | DESC] [NULLS FIRST | NULLS LAST]`.
    pub(super) fn sort_items(&mut self) -> Parsed<Vec<(Expression, SortDirection)>> {
        self.expect_keyword("ORDER")?;
        self.expect_keyword("BY")?;
        let mut items = Vec::new();
        loop {
            let expression = self.expression()?;
            items.push((expression, self.sort_direction()));
            if !self.eat_symbol(Symbol::Comma) {
                return Ok(items);
            }
        }
    }

    /// `[ASC | DESC] [NULLS FIRST | NULLS LAST]`; the `NULLS` clause is read and dropped.
    fn sort_direction(&mut self) -> SortDirection {
        let direction = if self.eat_keyword("DESC") {
            SortDirection::Descending
        } else {
            self.eat_keyword("ASC");
            SortDirection::Ascending
        };
        if self.keyword_at(0, "NULLS")
            && (self.keyword_at(1, "FIRST") || self.keyword_at(1, "LAST"))
        {
            self.position += 2;
        }
        direction
    }

    /// An optional `FILTER (WHERE condition)`.
    pub(super) fn filter(&mut self) -> Parsed<Option<Box<Expression>>> {
        if !(self.keyword_at(0, "FILTER") && self.symbol_at(1, Symbol::LeftParenthesis)) {
            return Ok(None);
        }
        self.position += 2;
        self.expect_keyword("WHERE")?;
        let condition = self.expression()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(Some(Box::new(condition)))
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
