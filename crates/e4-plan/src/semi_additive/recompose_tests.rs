//! Unit tests of the recomposition of semi-additive components (E4-7.1).

use menura_model::expression::{AggregateCall, Expression, Literal, SimpleAggregate};
use menura_representations::logical_plan::AggregateComponent;
use menura_representations::names::ColumnName;

use super::recomposed;
use crate::columns::column_reference;
use crate::error::InvariantViolation;

fn component(call: AggregateCall) -> AggregateComponent {
    AggregateComponent {
        name: ColumnName::new("metric_1_component_1"),
        call,
        filter: Some(Expression::Literal(Literal::Boolean(true))),
    }
}

fn simple(function: SimpleAggregate) -> AggregateCall {
    AggregateCall::Simple {
        function,
        argument: Box::new(column_reference("BALANCE").expect("a column")),
    }
}

fn recomposed_function(call: AggregateCall) -> Result<SimpleAggregate, InvariantViolation> {
    let recomposed = recomposed(&component(call))?;
    assert_eq!(recomposed.filter, None);
    match recomposed.call {
        AggregateCall::Simple { function, argument } => {
            assert_eq!(
                *argument,
                column_reference("metric_1_component_1").expect("a column")
            );
            Ok(function)
        }
        _ => Err(InvariantViolation::ComponentMismatch),
    }
}

#[test]
fn e4_7_1_each_component_is_recomposed_by_its_own_function() {
    let cases = [
        (simple(SimpleAggregate::Sum), SimpleAggregate::Sum),
        (simple(SimpleAggregate::Count), SimpleAggregate::Sum),
        (AggregateCall::CountRows, SimpleAggregate::Sum),
        (simple(SimpleAggregate::Minimum), SimpleAggregate::Minimum),
        (simple(SimpleAggregate::Maximum), SimpleAggregate::Maximum),
    ];
    for (call, expected) in cases {
        assert_eq!(recomposed_function(call), Ok(expected));
    }
}

#[test]
fn e4_7_1_a_component_that_is_not_distributive_has_no_recomposition() {
    assert_eq!(
        recomposed_function(simple(SimpleAggregate::Average)),
        Err(InvariantViolation::ComponentMismatch)
    );
}
