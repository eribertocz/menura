//! The `Filter`, `Deduplicate` and `Aggregate` nodes of a branch (E4-4).

use menura_model::expression::Expression;
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use crate::error::PlanError;
use crate::typing::typed_column;

/// The `Filter` of `input` by `predicate`: it keeps the input's schema.
pub(crate) fn filter(input: PlanNode, predicate: Expression, origin: Origin) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Filter {
            input: Box::new(input),
            predicate,
        },
        schema,
        origin,
    )
}

/// The `Deduplicate` of `input` that keeps one row per (`keys` values, `identity`) pair
/// (E4-4.0e, E4-4.0f): its group is the key expressions, after time-grain truncation. It
/// keeps the input's schema.
pub(crate) fn deduplicate(
    input: PlanNode,
    keys: &[NamedExpression],
    identity: ColumnName,
    origin: Origin,
) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Deduplicate {
            input: Box::new(input),
            group: keys.iter().map(|key| key.expression.clone()).collect(),
            identity: vec![identity],
        },
        schema,
        origin,
    )
}

/// The `Aggregate` of `input` keyed by `keys`, with `components` (E4-4.0g).
pub(crate) fn aggregate(
    input: PlanNode,
    keys: Vec<NamedExpression>,
    components: Vec<AggregateComponent>,
    origin: &Origin,
) -> Result<PlanNode, PlanError> {
    let fail = |violation| PlanError::invariant(violation, origin);
    let mut columns = Vec::new();
    for key in &keys {
        columns
            .push(typed_column(key.name.clone(), &key.expression, input.schema()).map_err(fail)?);
    }
    for component in &components {
        let expression = component_expression(component);
        columns
            .push(typed_column(component.name.clone(), &expression, input.schema()).map_err(fail)?);
    }
    Ok(PlanNode::new(
        Operation::Aggregate {
            input: Box::new(input),
            keys,
            components,
        },
        OutputSchema::new(columns),
        origin.clone(),
    ))
}

/// The aggregate expression of `component`.
pub(crate) fn component_expression(component: &AggregateComponent) -> Expression {
    Expression::Aggregate {
        call: component.call.clone(),
        filter: component.filter.clone().map(Box::new),
    }
}
