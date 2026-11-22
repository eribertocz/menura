//! Builders of the expression trees of the summary tests.

use menura_model::expression::{
    AggregateCall, BinaryOperator, CastTarget, DateUnit, Expression, Identifier, Literal,
    Reference, SimpleAggregate,
};

/// The reference to the plan or catalog column `name`, as E4 writes it: delimited.
pub(in super::super) fn column(name: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::delimited(name).expect("a name"),
    })
}

/// The field reference `dataset.field`, as a `base_aggregate` writes it.
pub(in super::super) fn qualified(dataset: &str, field: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: Identifier::regular(dataset),
        name: Identifier::regular(field).expect("a name"),
    })
}

pub(in super::super) fn text(value: &str) -> Expression {
    Expression::Literal(Literal::Text(value.to_owned()))
}

pub(in super::super) fn binary(
    operator: BinaryOperator,
    left: Expression,
    right: Expression,
) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

/// `column = 'value'`.
pub(in super::super) fn equals(name: &str, value: &str) -> Expression {
    binary(BinaryOperator::Equal, column(name), text(value))
}

/// `column IS NOT NULL`.
pub(in super::super) fn is_set(name: &str) -> Expression {
    Expression::IsNull {
        operand: Box::new(column(name)),
        negated: true,
    }
}

/// `DATE_TRUNC(unit, column)`.
pub(in super::super) fn truncated(unit: DateUnit, name: &str) -> Expression {
    Expression::DateTrunc {
        unit,
        operand: Box::new(column(name)),
    }
}

/// `CAST(operand AS target)`.
pub(in super::super) fn cast(operand: Expression, target: CastTarget) -> Expression {
    Expression::Cast {
        operand: Box::new(operand),
        target,
    }
}

/// `function(argument)`.
pub(in super::super) fn call(function: SimpleAggregate, argument: Expression) -> AggregateCall {
    AggregateCall::Simple {
        function,
        argument: Box::new(argument),
    }
}

/// The aggregate expression `function(argument)`.
pub(in super::super) fn aggregate_of(
    function: SimpleAggregate,
    argument: Expression,
) -> Expression {
    Expression::Aggregate {
        call: call(function, argument),
        filter: None,
    }
}
