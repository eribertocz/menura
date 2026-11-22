//! The plan of a semi-additive branch (E4-7): its rows reduced by a `Snapshot` to one value
//! per (result group, entity) under the metric's rule, then combined across entities by an
//! `Aggregate` that recomposes each component with its own function (E4-7.1). The metric's
//! expression is evaluated last, over the recomposed components, like every metric (E4-5.0c).

mod recompose;
mod rule;
mod time;

use menura_model::internal_model::{Metric, SemiAdditive};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode, SnapshotOver,
    SnapshotRule,
};
use menura_representations::origin::Origin;

use crate::columns::column_reference;
use crate::dimensions::ComputedDimensions;
use crate::error::{InvariantViolation, PlanError};
use crate::nodes::{aggregate, component_expression};
use crate::query_filter::{RangeShift, query_predicate};
use crate::rows::{BranchSource, RowsRequest, branch_rows};
use crate::typing::typed_column;

pub(crate) use rule::over_type;

/// A semi-additive branch to plan, with its keys and components over its home `Scan`.
pub(crate) struct SemiAdditiveBranch<'input> {
    /// Where its rows are read from.
    pub(crate) source: BranchSource<'input>,
    /// The query.
    pub(crate) query: &'input BoundQuery,
    /// The semi-additive metric whose components the branch holds (E3-4.1).
    pub(crate) metric: &'input Metric,
    /// Its declaration.
    pub(crate) declaration: &'input SemiAdditive,
    /// The `Aggregate` keys of the branch.
    pub(crate) keys: Vec<NamedExpression>,
    /// The expression dimensions the keys read, when the query has any.
    pub(crate) computed: Option<ComputedDimensions>,
    /// Its components.
    pub(crate) components: Vec<AggregateComponent>,
    /// The shift of the `date_range` of a `prior_period` context query (E4-8.4).
    pub(crate) range_shift: Option<RangeShift<'input>>,
}

impl SemiAdditiveBranch<'_> {
    /// The origin of the nodes the rule adds: the metric.
    fn origin(&self) -> Origin {
        Origin::Model(self.metric.location.clone())
    }
}

/// The plan of `branch`, up to its `Aggregate`: the `Snapshot` of its rows, then the
/// recomposition of each component across entities, by the same keys (E4-7.1, ADD-4.6).
///
/// # Errors
///
/// `ESEC-02` and `EQRY-03` from the row policies (E4-6.9), and an internal error for an input
/// the stages before E4 never produce.
pub(crate) fn semi_additive_plan(branch: &SemiAdditiveBranch<'_>) -> Result<PlanNode, PlanError> {
    let origin = branch.origin();
    let fail = |violation| PlanError::invariant(violation, &origin);
    let home = branch.source.home;
    let declaration = branch.declaration;
    let entity = rule::entity_columns(declaration, home).map_err(fail)?;
    let over = rule::over_column(declaration, home).map_err(fail)?;
    let input = branch_rows(
        &branch.source,
        RowsRequest {
            keys: &branch.keys,
            computed: branch.computed.as_ref(),
            components: &branch.components,
            fields: &rule::snapshot_fields(declaration),
            predicate: query_predicate(branch.query, branch.source.layout, branch.range_shift)?,
            replaced_home: None,
        },
    )?;
    let over = match declaration.snapshot_grain {
        Some(grain) => SnapshotOver::Time(time::time_snapshot(branch, &input, over, grain)?),
        None => SnapshotOver::Field(over),
    };
    let rule = SnapshotRule {
        metric: branch.metric.name.clone(),
        keys: branch.keys.clone(),
        entity,
        components: branch.components.clone(),
        aggregation: declaration.aggregation,
        nulls: declaration.nulls,
        over,
    };
    let snapshot = snapshot(input, rule, &origin).map_err(fail)?;
    recomposition(snapshot, branch, &origin)
}

/// The `Aggregate` over `snapshot` by the keys of `branch`, which recomposes each component
/// across entities (E4-7.1, ADD-4.6).
fn recomposition(
    snapshot: PlanNode,
    branch: &SemiAdditiveBranch<'_>,
    origin: &Origin,
) -> Result<PlanNode, PlanError> {
    let fail = |violation| PlanError::invariant(violation, origin);
    let keys = branch
        .keys
        .iter()
        .map(|key| {
            Ok(NamedExpression {
                name: key.name.clone(),
                expression: column_reference(key.name.as_str())?,
                derived: None,
            })
        })
        .collect::<Result<Vec<_>, InvariantViolation>>()
        .map_err(fail)?;
    let components = branch
        .components
        .iter()
        .map(recompose::recomposed)
        .collect::<Result<Vec<_>, _>>()
        .map_err(fail)?;
    aggregate(snapshot, keys, components, origin)
}

/// The `Snapshot` of `input` under `rule`: its output is the keys, the entity columns and the
/// component columns, typed over `input` (E4-3.13).
fn snapshot(
    input: PlanNode,
    rule: SnapshotRule,
    origin: &Origin,
) -> Result<PlanNode, InvariantViolation> {
    let schema = input.schema();
    let mut columns = Vec::new();
    for named in rule.keys.iter().chain(&rule.entity) {
        columns.push(typed_column(named.name.clone(), &named.expression, schema)?);
    }
    for component in &rule.components {
        let expression = component_expression(component);
        columns.push(typed_column(component.name.clone(), &expression, schema)?);
    }
    Ok(PlanNode::new(
        Operation::Snapshot {
            input: Box::new(input),
            rule,
        },
        OutputSchema::new(columns),
        origin.clone(),
    ))
}
