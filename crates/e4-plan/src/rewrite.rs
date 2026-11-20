//! Rebuilds an expression tree with some of its nodes replaced.

use menura_model::expression::{AggregateCall, CaseBranch, Expression};

/// `expression` with every node for which `replace` gives a replacement replaced by it, the
/// others rebuilt from their rewritten subexpressions. `replace` sees each node before its
/// subexpressions, in written order, and a replaced node's subexpressions are not visited.
///
/// # Errors
///
/// The first error `replace` returns.
pub(crate) fn rewrite<Error>(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Result<Option<Expression>, Error>,
) -> Result<Expression, Error> {
    if let Some(replaced) = replace(expression)? {
        return Ok(replaced);
    }
    if let Some(rewritten) = rewrite_operator(expression, replace)? {
        return Ok(rewritten);
    }
    if let Some(rewritten) = rewrite_test(expression, replace)? {
        return Ok(rewritten);
    }
    if let Some(rewritten) = rewrite_conversion(expression, replace)? {
        return Ok(rewritten);
    }
    Ok(match expression {
        Expression::Case {
            operand,
            branches,
            otherwise,
        } => rewrite_case(operand.as_deref(), branches, otherwise.as_deref(), replace)?,
        Expression::Function {
            function,
            arguments,
        } => Expression::Function {
            function: *function,
            arguments: all(arguments, replace)?,
        },
        Expression::Aggregate { call, filter } => Expression::Aggregate {
            call: rewrite_call(call, replace)?,
            filter: filter
                .as_deref()
                .map(|filter| boxed(filter, replace))
                .transpose()?,
        },
        // A leaf: a literal, a reference, `AGG(…)`, `USER_ATTRIBUTE(…)` or a current time.
        _ => expression.clone(),
    })
}

/// `expression` rewritten when it is an operator node; `None` otherwise.
fn rewrite_operator<Error>(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Result<Option<Expression>, Error>,
) -> Result<Option<Expression>, Error> {
    Ok(Some(match expression {
        Expression::Unary { operator, operand } => Expression::Unary {
            operator: *operator,
            operand: boxed(operand, replace)?,
        },
        Expression::Binary {
            operator,
            left,
            right,
        } => Expression::Binary {
            operator: *operator,
            left: boxed(left, replace)?,
            right: boxed(right, replace)?,
        },
        Expression::IsNull { operand, negated } => Expression::IsNull {
            operand: boxed(operand, replace)?,
            negated: *negated,
        },
        Expression::IsDistinctFrom {
            left,
            right,
            negated,
        } => Expression::IsDistinctFrom {
            left: boxed(left, replace)?,
            right: boxed(right, replace)?,
            negated: *negated,
        },
        _ => return Ok(None),
    }))
}

/// `expression` rewritten when it tests a value against a list, a range or a pattern;
/// `None` otherwise.
fn rewrite_test<Error>(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Result<Option<Expression>, Error>,
) -> Result<Option<Expression>, Error> {
    Ok(Some(match expression {
        Expression::InList {
            operand,
            list,
            negated,
        } => Expression::InList {
            operand: boxed(operand, replace)?,
            list: all(list, replace)?,
            negated: *negated,
        },
        Expression::Between {
            operand,
            low,
            high,
            negated,
        } => Expression::Between {
            operand: boxed(operand, replace)?,
            low: boxed(low, replace)?,
            high: boxed(high, replace)?,
            negated: *negated,
        },
        Expression::Like {
            operand,
            pattern,
            case,
            negated,
        } => Expression::Like {
            operand: boxed(operand, replace)?,
            pattern: boxed(pattern, replace)?,
            case: *case,
            negated: *negated,
        },
        _ => return Ok(None),
    }))
}

/// `expression` rewritten when it is a cast or a date and time node; `None` otherwise.
fn rewrite_conversion<Error>(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Result<Option<Expression>, Error>,
) -> Result<Option<Expression>, Error> {
    Ok(Some(match expression {
        Expression::Cast { operand, target } => Expression::Cast {
            operand: boxed(operand, replace)?,
            target: *target,
        },
        Expression::Extract { part, operand } => Expression::Extract {
            part: *part,
            operand: boxed(operand, replace)?,
        },
        Expression::DateTrunc { unit, operand } => Expression::DateTrunc {
            unit: *unit,
            operand: boxed(operand, replace)?,
        },
        Expression::DateAdd {
            unit,
            amount,
            operand,
        } => Expression::DateAdd {
            unit: *unit,
            amount: boxed(amount, replace)?,
            operand: boxed(operand, replace)?,
        },
        Expression::DateDiff { unit, start, end } => Expression::DateDiff {
            unit: *unit,
            start: boxed(start, replace)?,
            end: boxed(end, replace)?,
        },
        _ => return Ok(None),
    }))
}

fn boxed<Error>(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Result<Option<Expression>, Error>,
) -> Result<Box<Expression>, Error> {
    rewrite(expression, replace).map(Box::new)
}

fn all<Error>(
    expressions: &[Expression],
    replace: &mut impl FnMut(&Expression) -> Result<Option<Expression>, Error>,
) -> Result<Vec<Expression>, Error> {
    expressions
        .iter()
        .map(|expression| rewrite(expression, replace))
        .collect()
}

fn rewrite_case<Error>(
    operand: Option<&Expression>,
    branches: &[CaseBranch],
    otherwise: Option<&Expression>,
    replace: &mut impl FnMut(&Expression) -> Result<Option<Expression>, Error>,
) -> Result<Expression, Error> {
    let operand = operand
        .map(|operand| rewrite(operand, replace).map(Box::new))
        .transpose()?;
    let branches = branches
        .iter()
        .map(|branch| {
            Ok(CaseBranch {
                condition: rewrite(&branch.condition, replace)?,
                result: rewrite(&branch.result, replace)?,
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    let otherwise = otherwise
        .map(|otherwise| rewrite(otherwise, replace).map(Box::new))
        .transpose()?;
    Ok(Expression::Case {
        operand,
        branches,
        otherwise,
    })
}

/// `call` with its arguments rewritten.
pub(crate) fn rewrite_call<Error>(
    call: &AggregateCall,
    replace: &mut impl FnMut(&Expression) -> Result<Option<Expression>, Error>,
) -> Result<AggregateCall, Error> {
    Ok(match call {
        AggregateCall::CountRows => AggregateCall::CountRows,
        AggregateCall::Simple { function, argument } => AggregateCall::Simple {
            function: *function,
            argument: Box::new(rewrite(argument, replace)?),
        },
        AggregateCall::Distinct { function, argument } => AggregateCall::Distinct {
            function: *function,
            argument: Box::new(rewrite(argument, replace)?),
        },
        AggregateCall::Percentile {
            function,
            fraction,
            ordering,
            direction,
        } => AggregateCall::Percentile {
            function: *function,
            fraction: Box::new(rewrite(fraction, replace)?),
            ordering: Box::new(rewrite(ordering, replace)?),
            direction: *direction,
        },
    })
}
