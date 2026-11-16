//! Operators by precedence, loosest first: `OR`, `AND`, `NOT`, predicates (comparisons, `IS`,
//! `IN`, `BETWEEN`, `LIKE`), `||`, `+ -`, `* / %`, prefix `+ -`.
//!
//! Comparisons and other predicates do not chain: `a = b = c` cannot be parsed. Binary
//! operators of one level associate to the left.

use semantic_compiler_model::expression::{BinaryOperator, Expression, LikeCase, UnaryOperator};

use super::{Parsed, Parser};
use crate::error::ExpressionErrorKind;
use crate::lexer::{Symbol, TokenKind};

fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

impl Parser<'_> {
    /// A complete expression.
    pub(super) fn expression(&mut self) -> Parsed<Expression> {
        let mut left = self.conjunction()?;
        while self.eat_keyword("OR") {
            left = binary(BinaryOperator::Or, left, self.conjunction()?);
        }
        Ok(left)
    }

    fn conjunction(&mut self) -> Parsed<Expression> {
        let mut left = self.negation()?;
        while self.eat_keyword("AND") {
            left = binary(BinaryOperator::And, left, self.negation()?);
        }
        Ok(left)
    }

    fn negation(&mut self) -> Parsed<Expression> {
        if self.eat_keyword("NOT") {
            return Ok(Expression::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(self.negation()?),
            });
        }
        self.predicate()
    }

    fn predicate(&mut self) -> Parsed<Expression> {
        let left = self.concatenation()?;
        if let Some(operator) = self.comparison_operator() {
            self.position += 1;
            self.reject_quantifier()?;
            return Ok(binary(operator, left, self.concatenation()?));
        }
        if self.eat_keyword("IS") {
            return self.is_test(left);
        }
        let negated = self.keyword_at(0, "NOT")
            && ["IN", "BETWEEN", "LIKE", "ILIKE"]
                .iter()
                .any(|keyword| self.keyword_at(1, keyword));
        if negated {
            self.position += 1;
        }
        if self.eat_keyword("IN") {
            return self.in_list(left, negated);
        }
        if self.eat_keyword("BETWEEN") {
            return self.between(left, negated);
        }
        let case = if self.eat_keyword("LIKE") {
            LikeCase::Sensitive
        } else if self.eat_keyword("ILIKE") {
            LikeCase::Insensitive
        } else {
            return Ok(left);
        };
        self.like(left, case, negated)
    }

    fn comparison_operator(&self) -> Option<BinaryOperator> {
        let TokenKind::Symbol(symbol) = self.peek_kind_at(0)? else {
            return None;
        };
        Some(match symbol {
            Symbol::Equal => BinaryOperator::Equal,
            Symbol::NotEqual => BinaryOperator::NotEqual,
            Symbol::Less => BinaryOperator::Less,
            Symbol::LessOrEqual => BinaryOperator::LessOrEqual,
            Symbol::Greater => BinaryOperator::Greater,
            Symbol::GreaterOrEqual => BinaryOperator::GreaterOrEqual,
            _ => return None,
        })
    }

    /// `= ANY (…)`, `= SOME (…)` and `= ALL (…)` are quantified comparisons over subqueries.
    fn reject_quantifier(&self) -> Parsed<()> {
        if ["ANY", "SOME", "ALL"]
            .iter()
            .any(|keyword| self.keyword_at(0, keyword))
        {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "a quantified comparison (ANY, SOME, ALL)",
            )));
        }
        Ok(())
    }

    /// After `IS`: `[NOT] NULL` or `[NOT] DISTINCT FROM operand`.
    fn is_test(&mut self, left: Expression) -> Parsed<Expression> {
        let negated = self.eat_keyword("NOT");
        if self.eat_keyword("NULL") {
            return Ok(Expression::IsNull {
                operand: Box::new(left),
                negated,
            });
        }
        if self.eat_keyword("DISTINCT") {
            self.expect_keyword("FROM")?;
            return Ok(Expression::IsDistinctFrom {
                left: Box::new(left),
                right: Box::new(self.concatenation()?),
                negated,
            });
        }
        if ["TRUE", "FALSE", "UNKNOWN"]
            .iter()
            .any(|keyword| self.keyword_at(0, keyword))
        {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "IS TRUE, IS FALSE and IS UNKNOWN",
            )));
        }
        Err(self.unexpected("NULL or DISTINCT FROM"))
    }

    fn in_list(&mut self, operand: Expression, negated: bool) -> Parsed<Expression> {
        self.expect_symbol(Symbol::LeftParenthesis)?;
        let list = self.expression_list()?;
        self.expect_symbol(Symbol::RightParenthesis)?;
        Ok(Expression::InList {
            operand: Box::new(operand),
            list,
            negated,
        })
    }

    /// One or more expressions separated by commas.
    pub(super) fn expression_list(&mut self) -> Parsed<Vec<Expression>> {
        let mut list = vec![self.expression()?];
        while self.eat_symbol(Symbol::Comma) {
            list.push(self.expression()?);
        }
        Ok(list)
    }

    fn between(&mut self, operand: Expression, negated: bool) -> Parsed<Expression> {
        if self.keyword_at(0, "SYMMETRIC") || self.keyword_at(0, "ASYMMETRIC") {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct(
                "BETWEEN SYMMETRIC and BETWEEN ASYMMETRIC",
            )));
        }
        let low = self.concatenation()?;
        self.expect_keyword("AND")?;
        let high = self.concatenation()?;
        Ok(Expression::Between {
            operand: Box::new(operand),
            low: Box::new(low),
            high: Box::new(high),
            negated,
        })
    }

    fn like(&mut self, operand: Expression, case: LikeCase, negated: bool) -> Parsed<Expression> {
        let pattern = self.concatenation()?;
        if self.keyword_at(0, "ESCAPE") {
            return Err(self.error_here(ExpressionErrorKind::UnsupportedConstruct("LIKE … ESCAPE")));
        }
        Ok(Expression::Like {
            operand: Box::new(operand),
            pattern: Box::new(pattern),
            case,
            negated,
        })
    }

    fn concatenation(&mut self) -> Parsed<Expression> {
        let mut left = self.additive()?;
        while self.eat_symbol(Symbol::Concatenate) {
            left = binary(BinaryOperator::Concatenate, left, self.additive()?);
        }
        Ok(left)
    }

    fn additive(&mut self) -> Parsed<Expression> {
        let mut left = self.multiplicative()?;
        loop {
            let operator = if self.eat_symbol(Symbol::Plus) {
                BinaryOperator::Add
            } else if self.eat_symbol(Symbol::Minus) {
                BinaryOperator::Subtract
            } else {
                return Ok(left);
            };
            left = binary(operator, left, self.multiplicative()?);
        }
    }

    fn multiplicative(&mut self) -> Parsed<Expression> {
        let mut left = self.prefixed()?;
        loop {
            let operator = if self.eat_symbol(Symbol::Star) {
                BinaryOperator::Multiply
            } else if self.eat_symbol(Symbol::Slash) {
                BinaryOperator::Divide
            } else if self.eat_symbol(Symbol::Percent) {
                BinaryOperator::Modulo
            } else {
                return Ok(left);
            };
            left = binary(operator, left, self.prefixed()?);
        }
    }

    fn prefixed(&mut self) -> Parsed<Expression> {
        let operator = if self.eat_symbol(Symbol::Plus) {
            UnaryOperator::Plus
        } else if self.eat_symbol(Symbol::Minus) {
            UnaryOperator::Minus
        } else {
            return self.primary();
        };
        Ok(Expression::Unary {
            operator,
            operand: Box::new(self.prefixed()?),
        })
    }
}
