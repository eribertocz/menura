//! The decomposability class of an aggregate (E0G-3), whether it can be recomposed from partial
//! results (E0G-3.1), and the additivity deduced from the classes (E0G-4).

use menura_model::expression::{AggregateCall, SimpleAggregate};

use super::output::{Additivity, Component, DecomposabilityClass};

/// The class of `call` (E0G-3). The `FILTER` is not read: it does not change the class
/// (E0G-3.2).
pub(super) fn class(call: &AggregateCall) -> DecomposabilityClass {
    match call {
        AggregateCall::CountRows => DecomposabilityClass::Distributive,
        AggregateCall::Simple { function, .. } => simple_class(*function),
        AggregateCall::Distinct { .. } | AggregateCall::Percentile { .. } => {
            DecomposabilityClass::Holistic
        }
    }
}

fn simple_class(function: SimpleAggregate) -> DecomposabilityClass {
    match function {
        SimpleAggregate::Sum
        | SimpleAggregate::Count
        | SimpleAggregate::Minimum
        | SimpleAggregate::Maximum => DecomposabilityClass::Distributive,
        SimpleAggregate::Average
        | SimpleAggregate::StandardDeviation
        | SimpleAggregate::PopulationStandardDeviation
        | SimpleAggregate::SampleStandardDeviation
        | SimpleAggregate::Variance
        | SimpleAggregate::PopulationVariance
        | SimpleAggregate::SampleVariance => DecomposabilityClass::Algebraic,
        SimpleAggregate::Median => DecomposabilityClass::Holistic,
    }
}

/// Whether `call` can be recomposed from partial results (E0G-3): every distributive
/// aggregate, and of the algebraic ones only `AVG`, as `SUM` / `COUNT`.
pub(super) fn is_recomposable(call: &AggregateCall) -> bool {
    match class(call) {
        DecomposabilityClass::Distributive => true,
        DecomposabilityClass::Algebraic => matches!(
            call,
            AggregateCall::Simple {
                function: SimpleAggregate::Average,
                ..
            }
        ),
        DecomposabilityClass::Holistic => false,
    }
}

/// The additivity deduced for a metric of `components` (E0G-4): additive when every component
/// is distributive, otherwise non-additive.
pub(super) fn deduced_additivity(components: &[Component]) -> Additivity {
    if components
        .iter()
        .all(|component| component.class == DecomposabilityClass::Distributive)
    {
        Additivity::Additive
    } else {
        Additivity::NonAdditive
    }
}
