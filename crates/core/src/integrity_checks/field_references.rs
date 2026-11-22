//! The field references of a `base_aggregate` (SUM-2.1) as plan columns: each `of.field`
//! reference becomes the unqualified reference to the `Scan` column of that field (E6-4.12).

use menura_contracts::normalize_unquoted_identifier;
use menura_model::expression::{AggregateCall, CaseBranch, Expression, Identifier, Reference};
use menura_model::internal_model::NormalizedName;

/// `expression` with every field reference replaced by the reference to the column of that
/// field, and the fields it reads, each once, in written order.
pub(super) fn over_columns(expression: &Expression) -> (Expression, Vec<NormalizedName>) {
    let mut read = Vec::new();
    let mapped = map(expression, &mut read);
    (mapped, read)
}

/// The normalized name of the field `identifier` names (E0D-1.1).
fn field_name(identifier: &Identifier) -> NormalizedName {
    if identifier.is_delimited() {
        NormalizedName::new(identifier.text())
    } else {
        NormalizedName::new(&normalize_unquoted_identifier(identifier.text()))
    }
}

fn map(expression: &Expression, read: &mut Vec<NormalizedName>) -> Expression {
    let mut each = |inner: &Expression| Box::new(map(inner, read));
    match expression {
        Expression::Reference(reference) => column_of(expression, reference, read),
        Expression::Unary { operator, operand } => Expression::Unary {
            operator: *operator,
            operand: each(operand),
        },
        Expression::Binary {
            operator,
            left,
            right,
        } => Expression::Binary {
            operator: *operator,
            left: each(left),
            right: each(right),
        },
        Expression::Cast { operand, target } => Expression::Cast {
            operand: each(operand),
            target: *target,
        },
        Expression::Extract { part, operand } => Expression::Extract {
            part: *part,
            operand: each(operand),
        },
        Expression::DateTrunc { unit, operand } => Expression::DateTrunc {
            unit: *unit,
            operand: each(operand),
        },
        _ => map_test(expression, read),
    }
}

/// The reference to the column of the field `reference` names, which it adds to `read`.
fn column_of(
    expression: &Expression,
    reference: &Reference,
    read: &mut Vec<NormalizedName>,
) -> Expression {
    let field = field_name(&reference.name);
    let column = Identifier::delimited(field.as_str()).map_or_else(
        || expression.clone(),
        |name| {
            Expression::Reference(Reference {
                qualifier: None,
                name,
            })
        },
    );
    if !read.contains(&field) {
        read.push(field);
    }
    column
}

fn map_test(expression: &Expression, read: &mut Vec<NormalizedName>) -> Expression {
    let mut each = |inner: &Expression| Box::new(map(inner, read));
    match expression {
        Expression::IsNull { operand, negated } => Expression::IsNull {
            operand: each(operand),
            negated: *negated,
        },
        Expression::IsDistinctFrom {
            left,
            right,
            negated,
        } => Expression::IsDistinctFrom {
            left: each(left),
            right: each(right),
            negated: *negated,
        },
        Expression::Between {
            operand,
            low,
            high,
            negated,
        } => Expression::Between {
            operand: each(operand),
            low: each(low),
            high: each(high),
            negated: *negated,
        },
        Expression::Like {
            operand,
            pattern,
            case,
            negated,
        } => Expression::Like {
            operand: each(operand),
            pattern: each(pattern),
            case: *case,
            negated: *negated,
        },
        _ => map_dates(expression, read),
    }
}

fn map_dates(expression: &Expression, read: &mut Vec<NormalizedName>) -> Expression {
    let mut each = |inner: &Expression| Box::new(map(inner, read));
    match expression {
        Expression::DateAdd {
            unit,
            amount,
            operand,
        } => Expression::DateAdd {
            unit: *unit,
            amount: each(amount),
            operand: each(operand),
        },
        Expression::DateDiff { unit, start, end } => Expression::DateDiff {
            unit: *unit,
            start: each(start),
            end: each(end),
        },
        _ => map_compound(expression, read),
    }
}

fn map_compound(expression: &Expression, read: &mut Vec<NormalizedName>) -> Expression {
    match expression {
        Expression::InList {
            operand,
            list,
            negated,
        } => Expression::InList {
            operand: Box::new(map(operand, read)),
            list: list.iter().map(|item| map(item, read)).collect(),
            negated: *negated,
        },
        Expression::Case {
            operand,
            branches,
            otherwise,
        } => Expression::Case {
            operand: operand.as_deref().map(|inner| Box::new(map(inner, read))),
            branches: branches
                .iter()
                .map(|branch| CaseBranch {
                    condition: map(&branch.condition, read),
                    result: map(&branch.result, read),
                })
                .collect(),
            otherwise: otherwise.as_deref().map(|inner| Box::new(map(inner, read))),
        },
        Expression::Function {
            function,
            arguments,
        } => Expression::Function {
            function: *function,
            arguments: arguments
                .iter()
                .map(|argument| map(argument, read))
                .collect(),
        },
        Expression::Aggregate { call, filter } => Expression::Aggregate {
            call: map_call(call, read),
            filter: filter.as_deref().map(|inner| Box::new(map(inner, read))),
        },
        // A leaf without a field: a literal, `AGG(…)`, `USER_ATTRIBUTE(…)` or a current time.
        _ => expression.clone(),
    }
}

fn map_call(call: &AggregateCall, read: &mut Vec<NormalizedName>) -> AggregateCall {
    match call {
        AggregateCall::CountRows => AggregateCall::CountRows,
        AggregateCall::Simple { function, argument } => AggregateCall::Simple {
            function: *function,
            argument: Box::new(map(argument, read)),
        },
        AggregateCall::Distinct { function, argument } => AggregateCall::Distinct {
            function: *function,
            argument: Box::new(map(argument, read)),
        },
        AggregateCall::Percentile {
            function,
            fraction,
            ordering,
            direction,
        } => AggregateCall::Percentile {
            function: *function,
            fraction: Box::new(map(fraction, read)),
            ordering: Box::new(map(ordering, read)),
            direction: *direction,
        },
    }
}

#[cfg(test)]
mod tests {
    use menura_model::expression::{AggregateCall, Expression, Identifier, Reference};
    use menura_model::internal_model::NormalizedName;

    use super::over_columns;

    fn qualified(dataset: &str, field: &str) -> Expression {
        Expression::Reference(Reference {
            qualifier: Identifier::regular(dataset),
            name: Identifier::regular(field).unwrap(),
        })
    }

    fn column(name: &str) -> Expression {
        Expression::Reference(Reference {
            qualifier: None,
            name: Identifier::delimited(name).unwrap(),
        })
    }

    #[test]
    fn chk_5_5_a_base_aggregate_reads_the_columns_of_its_fields() {
        let sum = |argument| Expression::Aggregate {
            call: AggregateCall::Simple {
                function: menura_model::expression::SimpleAggregate::Sum,
                argument: Box::new(argument),
            },
            filter: Some(Box::new(Expression::IsNull {
                operand: Box::new(qualified("orders", "region")),
                negated: true,
            })),
        };
        let (mapped, read) = over_columns(&sum(qualified("orders", "amount")));
        let expected = Expression::Aggregate {
            call: AggregateCall::Simple {
                function: menura_model::expression::SimpleAggregate::Sum,
                argument: Box::new(column("AMOUNT")),
            },
            filter: Some(Box::new(Expression::IsNull {
                operand: Box::new(column("REGION")),
                negated: true,
            })),
        };
        assert_eq!(mapped, expected);
        assert_eq!(
            read,
            [NormalizedName::new("AMOUNT"), NormalizedName::new("REGION")]
        );
    }
}
