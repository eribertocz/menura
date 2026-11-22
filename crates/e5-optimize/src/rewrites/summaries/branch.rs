//! The parts of a branch plan (E4-4): its home `Scan`, row-policy nodes, joins, query filter,
//! computed dimensions, `Deduplicate` and `Aggregate`.

use menura_model::expression::{BinaryOperator, Expression};
use menura_model::internal_model::NormalizedName;
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{
    AggregateComponent, JoinCondition, NamedExpression, Operation, PlanNode,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

/// A node of the branch with the part of its operation that reads columns of its input.
#[derive(Debug)]
pub(super) struct Part<'plan, Value: ?Sized> {
    /// The node.
    pub(super) node: &'plan PlanNode,
    /// What it reads.
    pub(super) value: &'plan Value,
}

// Written by hand: a derive would require `Value: Clone`, which slices are not.
impl<Value: ?Sized> Clone for Part<'_, Value> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Value: ?Sized> Copy for Part<'_, Value> {}

/// A branch plan in the order of E4-4, from the home `Scan` up to its `Aggregate`.
#[derive(Debug, Clone)]
pub(super) struct BranchShape<'plan> {
    /// The home `Scan`.
    pub(super) scan: &'plan PlanNode,
    /// The home dataset.
    pub(super) home: &'plan NormalizedName,
    /// The `Filter` of the home dataset's own applicable row policies (E4-4.0b).
    pub(super) policy_filter: Option<Part<'plan, Expression>>,
    /// The propagation `SemiJoin` nodes of the home dataset, in plan order (E4-4.0b).
    pub(super) semi_joins: Vec<Part<'plan, [JoinCondition]>>,
    /// The `Join` nodes, in path order (E4-4.0c).
    pub(super) joins: Vec<Part<'plan, [JoinCondition]>>,
    /// The `Filter` of the query filters and `date_range` (E4-4.0d).
    pub(super) query_filter: Option<Part<'plan, Expression>>,
    /// The `Project` of the dimensions of the SQL form.
    pub(super) project: Option<Part<'plan, [NamedExpression]>>,
    /// Whether the branch has a `Deduplicate` (E4-4.0e).
    pub(super) deduplicate: bool,
    /// The `Aggregate` node.
    pub(super) aggregate: &'plan PlanNode,
    /// Its keys.
    pub(super) keys: &'plan [NamedExpression],
    /// Its components.
    pub(super) components: &'plan [AggregateComponent],
}

impl<'plan> BranchShape<'plan> {
    /// The parts of `branch`, an `Aggregate` over the nodes of E4-4; `None` for any other
    /// plan, such as a semi-additive branch with a `Snapshot` (E4-7).
    pub(super) fn of(branch: &'plan PlanNode) -> Option<Self> {
        let Operation::Aggregate {
            input,
            keys,
            components,
        } = branch.operation()
        else {
            return None;
        };
        let (node, deduplicate, project, query_filter) = upper_layers(input);
        let (node, joins) = joined(node);
        let (node, semi_joins, policy_filter) = policy_layers(node);
        let Operation::Scan { dataset, .. } = node.operation() else {
            return None;
        };
        Some(Self {
            scan: node,
            home: dataset,
            policy_filter,
            semi_joins,
            joins,
            query_filter,
            project,
            deduplicate,
            aggregate: branch,
            keys,
            components,
        })
    }

    /// Whether `column` is a column of the home `Scan`.
    pub(super) fn is_home_column(&self, column: &ColumnName) -> bool {
        self.scan
            .schema()
            .columns()
            .iter()
            .any(|scanned| &scanned.name == column)
    }

    /// The query filters and the `date_range` predicate of the query filter, apart: E4 writes
    /// the filters as one conjunction followed by the `date_range` predicate (E4-4.0d).
    pub(super) fn query_terms(
        &self,
        query: &BoundQuery,
    ) -> (Option<&'plan Expression>, Option<&'plan Expression>) {
        let Some(filter) = self.query_filter else {
            return (None, None);
        };
        let predicate = filter.value;
        if query.date_range().is_none() {
            return (Some(predicate), None);
        }
        if query.filters().is_empty() {
            return (None, Some(predicate));
        }
        match predicate {
            Expression::Binary {
                operator: BinaryOperator::And,
                left,
                right,
            } => (Some(left), Some(right)),
            _ => (Some(predicate), None),
        }
    }

    /// The dimensions of the SQL form that the `Project` computes: its expressions whose
    /// column its input does not have.
    pub(super) fn computed_dimensions(&self) -> Vec<&'plan NamedExpression> {
        let Some(project) = self.project else {
            return Vec::new();
        };
        let input = project.node.inputs();
        project
            .value
            .iter()
            .filter(|expression| {
                !input.iter().any(|input| {
                    input
                        .schema()
                        .columns()
                        .iter()
                        .any(|column| column.name == expression.name)
                })
            })
            .collect()
    }
}

/// The query layers over the joins, from the top: an optional `Deduplicate`, `Project` and
/// query-filter `Filter`; with the node below them.
#[expect(
    clippy::type_complexity,
    reason = "a private tuple of the layers, read once by `BranchShape::of`"
)]
fn upper_layers(
    top: &PlanNode,
) -> (
    &PlanNode,
    bool,
    Option<Part<'_, [NamedExpression]>>,
    Option<Part<'_, Expression>>,
) {
    let mut node = top;
    let deduplicate = matches!(node.operation(), Operation::Deduplicate { .. });
    if let Operation::Deduplicate { input, .. } = node.operation() {
        node = input;
    }
    let mut project = None;
    if let Operation::Project { input, expressions } = node.operation() {
        project = Some(Part {
            node,
            value: expressions.as_slice(),
        });
        node = input;
    }
    let mut query_filter = None;
    if let (Operation::Filter { input, predicate }, Origin::Query(_)) =
        (node.operation(), node.origin())
    {
        query_filter = Some(Part {
            node,
            value: predicate,
        });
        node = input;
    }
    (node, deduplicate, project, query_filter)
}

/// The `Join` nodes from `top` down, in path order, with the node below them.
fn joined(top: &PlanNode) -> (&PlanNode, Vec<Part<'_, [JoinCondition]>>) {
    let mut node = top;
    let mut joins = Vec::new();
    while let Operation::Join { left, on, .. } = node.operation() {
        joins.push(Part {
            node,
            value: on.as_slice(),
        });
        node = left;
    }
    joins.reverse();
    (node, joins)
}

/// The row-policy nodes of the home dataset from `top` down: its propagation `SemiJoin`
/// nodes, in plan order, and its own `Filter`; with the node below them.
#[expect(
    clippy::type_complexity,
    reason = "a private tuple of the layers, read once by `BranchShape::of`"
)]
fn policy_layers(
    top: &PlanNode,
) -> (
    &PlanNode,
    Vec<Part<'_, [JoinCondition]>>,
    Option<Part<'_, Expression>>,
) {
    let mut node = top;
    let mut semi_joins = Vec::new();
    while let Operation::SemiJoin { input, on, .. } = node.operation() {
        semi_joins.push(Part {
            node,
            value: on.as_slice(),
        });
        node = input;
    }
    semi_joins.reverse();
    let mut policy_filter = None;
    if let Operation::Filter { input, predicate } = node.operation() {
        policy_filter = Some(Part {
            node,
            value: predicate,
        });
        node = input;
    }
    (node, semi_joins, policy_filter)
}
