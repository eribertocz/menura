//! Rebuilds an expression tree with some of its nodes replaced, and lists what it reads.

use menura_contracts::normalize_unquoted_identifier;
use menura_model::expression::{AggregateCall, CaseBranch, Expression, Identifier, Reference};
use menura_representations::names::ColumnName;

/// `expression` with every node for which `replace` gives a replacement replaced by it, the
/// others rebuilt from their rewritten subexpressions. `replace` sees each node before its
/// subexpressions, and a replaced node's subexpressions are not visited.
pub(super) fn transform(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Option<Expression>,
) -> Expression {
    if let Some(replaced) = replace(expression) {
        return replaced;
    }
    if let Some(rebuilt) = transform_operator(expression, replace) {
        return rebuilt;
    }
    if let Some(rebuilt) = transform_test(expression, replace) {
        return rebuilt;
    }
    if let Some(rebuilt) = transform_conversion(expression, replace) {
        return rebuilt;
    }
    match expression {
        Expression::Case {
            operand,
            branches,
            otherwise,
        } => Expression::Case {
            operand: operand.as_deref().map(|operand| boxed(operand, replace)),
            branches: branches
                .iter()
                .map(|branch| CaseBranch {
                    condition: transform(&branch.condition, replace),
                    result: transform(&branch.result, replace),
                })
                .collect(),
            otherwise: otherwise
                .as_deref()
                .map(|otherwise| boxed(otherwise, replace)),
        },
        Expression::Function {
            function,
            arguments,
        } => Expression::Function {
            function: *function,
            arguments: all(arguments, replace),
        },
        Expression::Aggregate { call, filter } => Expression::Aggregate {
            call: transform_call(call, replace),
            filter: filter.as_deref().map(|filter| boxed(filter, replace)),
        },
        // A leaf: a literal, a reference, `AGG(…)`, `USER_ATTRIBUTE(…)` or a current time.
        _ => expression.clone(),
    }
}

fn transform_operator(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Option<Expression>,
) -> Option<Expression> {
    Some(match expression {
        Expression::Unary { operator, operand } => Expression::Unary {
            operator: *operator,
            operand: boxed(operand, replace),
        },
        Expression::Binary {
            operator,
            left,
            right,
        } => Expression::Binary {
            operator: *operator,
            left: boxed(left, replace),
            right: boxed(right, replace),
        },
        Expression::IsNull { operand, negated } => Expression::IsNull {
            operand: boxed(operand, replace),
            negated: *negated,
        },
        Expression::IsDistinctFrom {
            left,
            right,
            negated,
        } => Expression::IsDistinctFrom {
            left: boxed(left, replace),
            right: boxed(right, replace),
            negated: *negated,
        },
        _ => return None,
    })
}

fn transform_test(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Option<Expression>,
) -> Option<Expression> {
    Some(match expression {
        Expression::InList {
            operand,
            list,
            negated,
        } => Expression::InList {
            operand: boxed(operand, replace),
            list: all(list, replace),
            negated: *negated,
        },
        Expression::Between {
            operand,
            low,
            high,
            negated,
        } => Expression::Between {
            operand: boxed(operand, replace),
            low: boxed(low, replace),
            high: boxed(high, replace),
            negated: *negated,
        },
        Expression::Like {
            operand,
            pattern,
            case,
            negated,
        } => Expression::Like {
            operand: boxed(operand, replace),
            pattern: boxed(pattern, replace),
            case: *case,
            negated: *negated,
        },
        _ => return None,
    })
}

fn transform_conversion(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Option<Expression>,
) -> Option<Expression> {
    Some(match expression {
        Expression::Cast { operand, target } => Expression::Cast {
            operand: boxed(operand, replace),
            target: *target,
        },
        Expression::Extract { part, operand } => Expression::Extract {
            part: *part,
            operand: boxed(operand, replace),
        },
        Expression::DateTrunc { unit, operand } => Expression::DateTrunc {
            unit: *unit,
            operand: boxed(operand, replace),
        },
        Expression::DateAdd {
            unit,
            amount,
            operand,
        } => Expression::DateAdd {
            unit: *unit,
            amount: boxed(amount, replace),
            operand: boxed(operand, replace),
        },
        Expression::DateDiff { unit, start, end } => Expression::DateDiff {
            unit: *unit,
            start: boxed(start, replace),
            end: boxed(end, replace),
        },
        _ => return None,
    })
}

/// `call` with its arguments rewritten.
pub(super) fn transform_call(
    call: &AggregateCall,
    replace: &mut impl FnMut(&Expression) -> Option<Expression>,
) -> AggregateCall {
    match call {
        AggregateCall::CountRows => AggregateCall::CountRows,
        AggregateCall::Simple { function, argument } => AggregateCall::Simple {
            function: *function,
            argument: boxed(argument, replace),
        },
        AggregateCall::Distinct { function, argument } => AggregateCall::Distinct {
            function: *function,
            argument: boxed(argument, replace),
        },
        AggregateCall::Percentile {
            function,
            fraction,
            ordering,
            direction,
        } => AggregateCall::Percentile {
            function: *function,
            fraction: boxed(fraction, replace),
            ordering: boxed(ordering, replace),
            direction: *direction,
        },
    }
}

fn boxed(
    expression: &Expression,
    replace: &mut impl FnMut(&Expression) -> Option<Expression>,
) -> Box<Expression> {
    Box::new(transform(expression, replace))
}

fn all(
    expressions: &[Expression],
    replace: &mut impl FnMut(&Expression) -> Option<Expression>,
) -> Vec<Expression> {
    expressions
        .iter()
        .map(|expression| transform(expression, replace))
        .collect()
}

/// Whether `test` holds for some node of `expression`.
pub(super) fn any_node(
    expression: &Expression,
    test: &mut impl FnMut(&Expression) -> bool,
) -> bool {
    let mut found = false;
    transform(expression, &mut |node| {
        found = found || test(node);
        None
    });
    found
}

/// The references `expression` reads, in written order.
pub(super) fn references(expression: &Expression) -> Vec<Reference> {
    let mut read = Vec::new();
    transform(expression, &mut |node| {
        if let Expression::Reference(reference) = node {
            read.push(reference.clone());
        }
        None
    });
    read
}

/// The plan columns `expression` reads: its unqualified references, by name.
pub(super) fn columns_read(expression: &Expression) -> Vec<ColumnName> {
    references(expression)
        .into_iter()
        .filter(|reference| reference.qualifier.is_none())
        .map(|reference| ColumnName::new(reference.name.text()))
        .collect()
}

/// The normalized name of the field `identifier` names (E0D-1.1): a regular identifier in
/// upper case, a delimited one exactly.
pub(super) fn field_name(identifier: &Identifier) -> String {
    if identifier.is_delimited() {
        identifier.text().to_owned()
    } else {
        normalize_unquoted_identifier(identifier.text())
    }
}

/// The unqualified reference to the plan column `name`, as a delimited identifier so that it
/// names the column exactly; `None` for an empty name.
pub(super) fn column_reference(name: &str) -> Option<Expression> {
    Identifier::delimited(name).map(|name| {
        Expression::Reference(Reference {
            qualifier: None,
            name,
        })
    })
}

/// `expression` with every unqualified reference to a column that `rename` renames replaced
/// by the reference to its new name.
pub(super) fn renamed(
    expression: &Expression,
    rename: &impl Fn(&str) -> Option<ColumnName>,
) -> Expression {
    transform(expression, &mut |node| match node {
        Expression::Reference(reference) if reference.qualifier.is_none() => {
            rename(reference.name.text()).and_then(|name| column_reference(name.as_str()))
        }
        _ => None,
    })
}
