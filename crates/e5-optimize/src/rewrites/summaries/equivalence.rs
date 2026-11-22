//! SUM-3.1: each component of the branch matches summary measures after normalization and
//! recomposes exactly.

use menura_model::expression::{AggregateCall, Expression, SimpleAggregate};
use menura_model::internal_model::{NormalizedName, Summary};
use menura_representations::logical_plan::{AggregateComponent, Recomposition};
use menura_representations::names::ColumnName;

use super::expression_map::{column_reference, field_name, transform};

/// An aggregate over the columns of the home `Scan`: its call and its `FILTER (WHERE …)`.
pub(super) type ScanAggregate = (AggregateCall, Option<Expression>);

/// A recomposed component and the summary measures it reads.
#[derive(Debug, Clone)]
pub(super) struct Recomposed {
    /// The component over the summary (SUM-3.1).
    pub(super) component: AggregateComponent,
    /// The summary fields it reads.
    pub(super) reads: Vec<NormalizedName>,
}

/// `aggregate`, a normalized tree over fields of one dataset (E0H-1.3), over the columns of
/// its `Scan`: every field reference becomes the unqualified reference to the column of that
/// field, as E4 writes it. `None` when it is not an aggregate call.
pub(super) fn over_scan(aggregate: &Expression) -> Option<ScanAggregate> {
    let mapped = transform(aggregate, &mut |node| match node {
        Expression::Reference(reference) => column_reference(&field_name(&reference.name)),
        _ => None,
    });
    match mapped {
        Expression::Aggregate { call, filter } => Some((call, filter.map(|filter| *filter))),
        _ => None,
    }
}

/// The recomposed components of `components` over `summary` (SUM-3.1), in order; `None` when
/// one of them does not recompose.
pub(super) fn recompose(
    components: &[AggregateComponent],
    summary: &Summary,
) -> Option<Vec<Recomposed>> {
    let measures: Vec<(&NormalizedName, ScanAggregate)> = summary
        .measures
        .iter()
        .filter_map(|measure| {
            let tree = measure.base_aggregate.as_ref()?;
            Some((&measure.field, over_scan(tree)?))
        })
        .collect();
    components
        .iter()
        .map(|component| recompose_one(component, &measures))
        .collect()
}

fn recompose_one(
    component: &AggregateComponent,
    measures: &[(&NormalizedName, ScanAggregate)],
) -> Option<Recomposed> {
    let AggregateCall::Simple { function, argument } = &component.call else {
        return None;
    };
    let measure = |wanted| measure_of(measures, wanted, argument, component.filter.as_ref());
    let (outer, read, recomposition) = match function {
        SimpleAggregate::Sum => (
            SimpleAggregate::Sum,
            measure(SimpleAggregate::Sum)?,
            Recomposition::Sum,
        ),
        SimpleAggregate::Count => (
            SimpleAggregate::Sum,
            measure(SimpleAggregate::Count)?,
            Recomposition::Count,
        ),
        SimpleAggregate::Minimum => (
            SimpleAggregate::Minimum,
            measure(SimpleAggregate::Minimum)?,
            Recomposition::MinMax,
        ),
        SimpleAggregate::Maximum => (
            SimpleAggregate::Maximum,
            measure(SimpleAggregate::Maximum)?,
            Recomposition::MinMax,
        ),
        SimpleAggregate::Average => {
            let count = measure(SimpleAggregate::Count)?;
            let sum = measure(SimpleAggregate::Sum)?;
            return recomposed(
                component,
                SimpleAggregate::Sum,
                vec![sum, count.clone()],
                Recomposition::Average {
                    count: ColumnName::new(count.as_str()),
                },
            );
        }
        // Holistic and other aggregates never use summaries (SUM-3.1).
        _ => return None,
    };
    recomposed(component, outer, vec![read], recomposition)
}

/// `component` computed as `outer` over the first of `reads`, recomposed as `recomposition`.
fn recomposed(
    component: &AggregateComponent,
    outer: SimpleAggregate,
    reads: Vec<NormalizedName>,
    recomposition: Recomposition,
) -> Option<Recomposed> {
    let argument = column_reference(reads.first()?.as_str())?;
    Some(Recomposed {
        component: AggregateComponent {
            name: component.name.clone(),
            call: AggregateCall::Simple {
                function: outer,
                argument: Box::new(argument),
            },
            filter: None,
            recomposition: Some(recomposition),
        },
        reads,
    })
}

/// The summary field of the measure `wanted(argument)` with the filter `filter`.
fn measure_of(
    measures: &[(&NormalizedName, ScanAggregate)],
    wanted: SimpleAggregate,
    argument: &Expression,
    filter: Option<&Expression>,
) -> Option<NormalizedName> {
    let call = AggregateCall::Simple {
        function: wanted,
        argument: Box::new(argument.clone()),
    };
    measures
        .iter()
        .find(|(_, (measure_call, measure_filter))| {
            measure_call == &call && measure_filter.as_ref() == filter
        })
        .map(|(field, _)| (*field).clone())
}
