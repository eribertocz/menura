//! Unit tests of the expression walkers: `for_each_reference_mut` visits references in the
//! order of `nodes`, so a SQL-form expression is rewritten in the order row 3 resolved it.

use menura_model::expression::{
    AggregateCall, BinaryOperator, CaseBranch, DateUnit, Expression, Identifier, LikeCase,
    Reference, ScalarFunction, SimpleAggregate,
};

use crate::walk::{for_each_reference_mut, nodes};

/// The reference `name`, numbered by the test.
fn reference(name: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::regular(name).expect("an identifier"),
    })
}

fn boxed(name: &str) -> Box<Expression> {
    Box::new(reference(name))
}

/// One expression with a reference in every slot of every node with several children.
fn every_slot() -> Expression {
    let case = Expression::Case {
        operand: Some(boxed("r01")),
        branches: vec![CaseBranch {
            condition: reference("r02"),
            result: reference("r03"),
        }],
        otherwise: Some(boxed("r04")),
    };
    let calls = Expression::Function {
        function: ScalarFunction::Upper,
        arguments: arguments(),
    };
    Expression::Binary {
        operator: BinaryOperator::And,
        left: Box::new(case),
        right: Box::new(calls),
    }
}

/// The nodes `r05` to `r20` are written in, as the arguments of one call.
fn arguments() -> Vec<Expression> {
    vec![
        Expression::InList {
            operand: boxed("r05"),
            list: vec![reference("r06"), reference("r07")],
            negated: false,
        },
        Expression::Between {
            operand: boxed("r08"),
            low: boxed("r09"),
            high: boxed("r10"),
            negated: false,
        },
        Expression::Like {
            operand: boxed("r11"),
            pattern: boxed("r12"),
            case: LikeCase::Sensitive,
            negated: false,
        },
        Expression::DateAdd {
            unit: DateUnit::Year,
            amount: boxed("r13"),
            operand: boxed("r14"),
        },
        Expression::DateDiff {
            unit: DateUnit::Year,
            start: boxed("r15"),
            end: boxed("r16"),
        },
        Expression::IsDistinctFrom {
            left: boxed("r17"),
            right: boxed("r18"),
            negated: false,
        },
        Expression::Aggregate {
            call: AggregateCall::Simple {
                function: SimpleAggregate::Sum,
                argument: boxed("r19"),
            },
            filter: Some(Box::new(Expression::MetricAggregate(Reference {
                qualifier: None,
                name: Identifier::regular("r20").expect("an identifier"),
            }))),
        },
    ]
}

fn reference_name(node: &Expression) -> Option<String> {
    match node {
        Expression::Reference(reference) | Expression::MetricAggregate(reference) => {
            Some(reference.name.text().to_owned())
        }
        _ => None,
    }
}

#[test]
fn e2_5_3_the_mutable_walk_visits_references_in_the_order_of_nodes() {
    let mut expression = every_slot();
    let read: Vec<String> = nodes(&expression)
        .into_iter()
        .filter_map(reference_name)
        .collect();
    let mut visited = Vec::new();
    for_each_reference_mut(&mut expression, &mut |reference| {
        visited.push(reference.name.text().to_owned());
    });
    let expected: Vec<String> = (1..=20).map(|index| format!("r{index:02}")).collect();
    assert_eq!(read, expected);
    assert_eq!(visited, expected);
}
