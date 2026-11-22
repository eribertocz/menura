//! The recomposition of the components of a semi-additive metric across entities (E4-7.1,
//! E0G-3): each component by its own function, over the values a `Snapshot` keeps.

use menura_model::expression::{AggregateCall, SimpleAggregate};
use menura_representations::logical_plan::AggregateComponent;

use crate::columns::column_reference;
use crate::error::InvariantViolation;

#[cfg(test)]
#[path = "recompose_tests.rs"]
mod tests;

/// The component that combines the entity values of `component`, read from its column of the
/// `Snapshot` output, with its recomposition function: `SUM` and `COUNT` by `SUM`, `MIN` by
/// `MIN`, `MAX` by `MAX` (E0G-3). Its `FILTER` was applied by the `Snapshot`.
///
/// # Errors
///
/// [`InvariantViolation::ComponentMismatch`] for a component that is not distributive, which
/// E0G-4.1 rules out for a semi-additive metric.
pub(crate) fn recomposed(
    component: &AggregateComponent,
) -> Result<AggregateComponent, InvariantViolation> {
    let function = match &component.call {
        AggregateCall::CountRows => SimpleAggregate::Sum,
        AggregateCall::Simple { function, .. } => match function {
            SimpleAggregate::Sum | SimpleAggregate::Count => SimpleAggregate::Sum,
            SimpleAggregate::Minimum => SimpleAggregate::Minimum,
            SimpleAggregate::Maximum => SimpleAggregate::Maximum,
            _ => return Err(InvariantViolation::ComponentMismatch),
        },
        AggregateCall::Distinct { .. } | AggregateCall::Percentile { .. } => {
            return Err(InvariantViolation::ComponentMismatch);
        }
    };
    Ok(AggregateComponent {
        name: component.name.clone(),
        call: AggregateCall::Simple {
            function,
            argument: Box::new(column_reference(component.name.as_str())?),
        },
        filter: None,
        recomposition: None,
    })
}
