//! Types the scalar functions of E0F-10.2 (text) and E0F-10.4 (conditionals), and dispatches
//! the others.

use semantic_compiler_contracts::types::common::LogicalType;
use semantic_compiler_model::expression::{Expression, Literal, ScalarFunction};

use super::numbers::number_function;
use super::operators::case_rule;
use super::stop::{Outcome, Stop};
use super::typer::{ExpressionTyper, any_nullable, expect_text, expect_type};
use super::value_type::{Typed, big_integer, is_integer, timestamp};

impl ExpressionTyper<'_> {
    /// The type of a scalar function call.
    pub(super) fn function(
        &self,
        function: ScalarFunction,
        arguments: &[Expression],
    ) -> Outcome<Typed> {
        use ScalarFunction as Function;
        let typed = self.types_of(arguments)?;
        match function {
            Function::Year
            | Function::Quarter
            | Function::Month
            | Function::Day
            | Function::DayOfYear
            | Function::Hour
            | Function::Minute
            | Function::Second => date_part_function(function, &typed),
            Function::ToDate => text_function(&typed, LogicalType::Date),
            Function::ToTimestamp => text_function(&typed, timestamp()),
            Function::Concat
            | Function::Lower
            | Function::Upper
            | Function::Trim
            | Function::LeftTrim
            | Function::RightTrim
            | Function::Replace => text_function(&typed, LogicalType::Text),
            Function::Length | Function::Position => text_function(&typed, big_integer()),
            Function::Contains | Function::StartsWith | Function::EndsWith => {
                text_function(&typed, LogicalType::Boolean)
            }
            Function::Left | Function::Right | Function::Substring | Function::SplitPart => {
                slice_function(function, arguments, &typed)
            }
            Function::Greatest
            | Function::Least
            | Function::If
            | Function::Nullif
            | Function::Coalesce
            | Function::IfNull
            | Function::Nvl2 => self.conditional(function, arguments, &typed),
            _ => number_function(function, arguments, &typed),
        }
    }

    /// The conditional functions of E0F-10.4, `GREATEST` and `LEAST`.
    fn conditional(
        &self,
        function: ScalarFunction,
        arguments: &[Expression],
        typed: &[Typed],
    ) -> Outcome<Typed> {
        match (function, typed) {
            (ScalarFunction::If, [_, first, second]) => {
                let condition = arguments.first().ok_or(Stop::ReportedElsewhere)?;
                self.condition(condition, "the condition of IF")?;
                let results = [first.clone(), second.clone()];
                let value_type = case_rule(&results, "the results of IF")?;
                Ok(Typed {
                    value_type,
                    nullable: any_nullable(&results),
                })
            }
            (ScalarFunction::Nullif, [first, _]) => {
                case_rule(typed, "the arguments of NULLIF")?;
                Ok(Typed {
                    value_type: first.value_type.clone(),
                    nullable: true,
                })
            }
            (ScalarFunction::Nvl2, [_, first, second]) => {
                let results = [first.clone(), second.clone()];
                let value_type = case_rule(&results, "the results of NVL2")?;
                Ok(Typed {
                    value_type,
                    nullable: any_nullable(&results),
                })
            }
            (ScalarFunction::Coalesce | ScalarFunction::IfNull, _) => Ok(Typed {
                value_type: case_rule(typed, "the arguments of COALESCE")?,
                nullable: typed.iter().all(|typed| typed.nullable),
            }),
            _ => Ok(Typed {
                value_type: case_rule(typed, "the arguments of GREATEST and LEAST")?,
                nullable: any_nullable(typed),
            }),
        }
    }
}

/// A function whose arguments are all text, with result type `result`.
fn text_function(typed: &[Typed], result: LogicalType) -> Outcome<Typed> {
    for argument in typed {
        expect_text(argument, "an argument of a text function")?;
    }
    Ok(Typed::value(result, any_nullable(typed)))
}

/// `LEFT(s, n)`, `RIGHT(s, n)`, `SUBSTRING(s, start[, length])` and `SPLIT_PART(s, d, n)`:
/// text, then integers (after the delimiter of `SPLIT_PART`).
fn slice_function(
    function: ScalarFunction,
    arguments: &[Expression],
    typed: &[Typed],
) -> Outcome<Typed> {
    let texts = if function == ScalarFunction::SplitPart {
        2
    } else {
        1
    };
    for (index, argument) in typed.iter().enumerate() {
        if index < texts {
            expect_text(argument, "the text argument")?;
        } else {
            expect_type(argument, is_integer, "the position argument", "an integer")?;
        }
    }
    let last = arguments.last().ok_or(Stop::ReportedElsewhere)?;
    let nullable = match function {
        // `n` must be an integer literal `n ≥ 1` (E0F-10.2); any other is issue #126.
        ScalarFunction::SplitPart if !is_positive_literal(last) => return Err(Stop::Undecided),
        // `n < 0` and `length < 0` give NULL (E0F-10.2).
        ScalarFunction::Left | ScalarFunction::Right => !is_integer_literal(last),
        ScalarFunction::Substring => arguments.len() == 3 && !is_integer_literal(last),
        _ => false,
    };
    Ok(Typed::value(
        LogicalType::Text,
        nullable || any_nullable(typed),
    ))
}

/// Whether `expression` is an integer literal, which is never negative.
pub(super) fn is_integer_literal(expression: &Expression) -> bool {
    matches!(expression, Expression::Literal(Literal::Integer(_)))
}

fn is_positive_literal(expression: &Expression) -> bool {
    matches!(expression, Expression::Literal(Literal::Integer(number))
        if number.digits().bytes().any(|digit| digit != b'0'))
}

/// `YEAR`, `QUARTER`, `MONTH`, `DAY`, `DAYOFYEAR` over a date or timestamp, and `HOUR`,
/// `MINUTE`, `SECOND` over a time or timestamp (E0F-10.1).
fn date_part_function(function: ScalarFunction, typed: &[Typed]) -> Outcome<Typed> {
    let time_part = matches!(
        function,
        ScalarFunction::Hour | ScalarFunction::Minute | ScalarFunction::Second
    );
    let [operand] = typed else {
        return Err(Stop::ReportedElsewhere);
    };
    super::dates::expect_temporal(operand, time_part)?;
    Ok(Typed::value(big_integer(), operand.nullable))
}
