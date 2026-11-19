//! The normalized trees of the model (E0H-1.3): every identifier is stored as the delimited
//! identifier of its normalized text, so `amount`, `AMOUNT` and `"AMOUNT"` give one tree.

use semantic_compiler_model::expression::{
    AggregateCall, CaseBranch, Expression, Identifier, Reference,
};

use crate::expressions::{ExpressionDiagnosticCode, ExpressionPlace, parse_element_expression};
use crate::structure::CompiledVariant;

/// The normalized tree of `text`, or the query error of a text that does not parse.
pub(super) fn parse(
    text: &str,
    place: ExpressionPlace,
) -> Result<Expression, ExpressionDiagnosticCode> {
    parse_element_expression(text, place)
        .map(|tree| normalized(&tree))
        .map_err(|diagnostic| diagnostic.code)
}

/// The normalized tree of the compiled variant, when there is one and it parses.
pub(super) fn compiled(variant: CompiledVariant<'_>, place: ExpressionPlace) -> Option<Expression> {
    match variant {
        CompiledVariant::AnsiSql(text) | CompiledVariant::OssieSql2026(text) => {
            parse(text, place).ok()
        }
        CompiledVariant::Unusable => None,
    }
}

/// `tree` with every name normalized.
pub(super) fn normalized(tree: &Expression) -> Expression {
    let boxed = |expression: &Expression| Box::new(normalized(expression));
    match tree {
        Expression::Reference(reference) => Expression::Reference(reference_of(reference)),
        Expression::MetricAggregate(reference) => {
            Expression::MetricAggregate(reference_of(reference))
        }
        Expression::Unary { operator, operand } => Expression::Unary {
            operator: *operator,
            operand: boxed(operand),
        },
        Expression::Binary {
            operator,
            left,
            right,
        } => Expression::Binary {
            operator: *operator,
            left: boxed(left),
            right: boxed(right),
        },
        Expression::IsNull { operand, negated } => Expression::IsNull {
            operand: boxed(operand),
            negated: *negated,
        },
        Expression::IsDistinctFrom {
            left,
            right,
            negated,
        } => Expression::IsDistinctFrom {
            left: boxed(left),
            right: boxed(right),
            negated: *negated,
        },
        Expression::InList {
            operand,
            list,
            negated,
        } => Expression::InList {
            operand: boxed(operand),
            list: list.iter().map(normalized).collect(),
            negated: *negated,
        },
        _ => normalized_rest(tree),
    }
}

fn normalized_rest(tree: &Expression) -> Expression {
    let boxed = |expression: &Expression| Box::new(normalized(expression));
    match tree {
        Expression::Between {
            operand,
            low,
            high,
            negated,
        } => Expression::Between {
            operand: boxed(operand),
            low: boxed(low),
            high: boxed(high),
            negated: *negated,
        },
        Expression::Like {
            operand,
            pattern,
            case,
            negated,
        } => Expression::Like {
            operand: boxed(operand),
            pattern: boxed(pattern),
            case: *case,
            negated: *negated,
        },
        Expression::Case {
            operand,
            branches,
            otherwise,
        } => Expression::Case {
            operand: operand.as_deref().map(boxed),
            branches: branches
                .iter()
                .map(|branch| CaseBranch {
                    condition: normalized(&branch.condition),
                    result: normalized(&branch.result),
                })
                .collect(),
            otherwise: otherwise.as_deref().map(boxed),
        },
        Expression::Cast { operand, target } => Expression::Cast {
            operand: boxed(operand),
            target: *target,
        },
        _ => normalized_calls(tree),
    }
}

fn normalized_calls(tree: &Expression) -> Expression {
    let boxed = |expression: &Expression| Box::new(normalized(expression));
    match tree {
        Expression::Extract { part, operand } => Expression::Extract {
            part: *part,
            operand: boxed(operand),
        },
        Expression::DateTrunc { unit, operand } => Expression::DateTrunc {
            unit: *unit,
            operand: boxed(operand),
        },
        Expression::DateAdd {
            unit,
            amount,
            operand,
        } => Expression::DateAdd {
            unit: *unit,
            amount: boxed(amount),
            operand: boxed(operand),
        },
        Expression::DateDiff { unit, start, end } => Expression::DateDiff {
            unit: *unit,
            start: boxed(start),
            end: boxed(end),
        },
        Expression::Function {
            function,
            arguments,
        } => Expression::Function {
            function: *function,
            arguments: arguments.iter().map(normalized).collect(),
        },
        Expression::Aggregate { call, filter } => Expression::Aggregate {
            call: normalized_call(call),
            filter: filter.as_deref().map(boxed),
        },
        leaf => leaf.clone(),
    }
}

/// `call` with every name normalized.
pub(super) fn normalized_call(call: &AggregateCall) -> AggregateCall {
    let boxed = |expression: &Expression| Box::new(normalized(expression));
    match call {
        AggregateCall::CountRows => AggregateCall::CountRows,
        AggregateCall::Simple { function, argument } => AggregateCall::Simple {
            function: *function,
            argument: boxed(argument),
        },
        AggregateCall::Distinct { function, argument } => AggregateCall::Distinct {
            function: *function,
            argument: boxed(argument),
        },
        AggregateCall::Percentile {
            function,
            fraction,
            ordering,
            direction,
        } => AggregateCall::Percentile {
            function: *function,
            fraction: boxed(fraction),
            ordering: boxed(ordering),
            direction: *direction,
        },
    }
}

fn reference_of(reference: &Reference) -> Reference {
    Reference {
        qualifier: reference.qualifier.as_ref().map(identifier_of),
        name: identifier_of(&reference.name),
    }
}

/// The delimited identifier of the normalized text of `identifier`; an identifier is never
/// empty, so the fallback keeps it as it is.
fn identifier_of(identifier: &Identifier) -> Identifier {
    Identifier::delimited(&identifier.normalized()).unwrap_or_else(|| identifier.clone())
}
