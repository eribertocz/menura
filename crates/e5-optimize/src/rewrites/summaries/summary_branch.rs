//! E5-3.5a: the branch that reads a summary instead of its home dataset.

use menura_model::expression::Expression;
use menura_representations::logical_plan::{
    JoinCondition, NamedExpression, Operation, OutputSchema, PlanNode,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::branch::BranchShape;
use super::eligibility::SummaryUse;
use super::expression_map::{column_reference, renamed};

/// The branch `branch` reading the summary of `summary_use` (E5-3.5a): a `Scan` of the
/// summary, then the `Filter` of the base's applicable row policies and its propagation
/// `SemiJoin` nodes, its `Join` nodes, its query filter and `date_range` predicate and its
/// computed dimensions, every reference to a base field mapped to the summary field of the
/// dimension with that `base`, then the `Aggregate` of its keys, mapped, and the recomposed
/// components (SUM-3.1). The `Aggregate` keeps the output schema of the base branch, whose
/// types E6 enforces (SUM-3.8).
///
/// `None` when `branch` is not the branch `summary_use` was found eligible for: it is not an
/// `Aggregate` over the nodes of E4-4, or it has a `Deduplicate` (SUM-3.2).
pub(crate) fn summary_branch(branch: &PlanNode, summary_use: &SummaryUse) -> Option<PlanNode> {
    if summary_use.summary.as_str() != "" {
        return None;
    }
    let shape = BranchShape::of(branch)?;
    if shape.deduplicate || shape.components.len() != summary_use.components.len() {
        return None;
    }
    let mapping = Mapping { summary_use };
    let mut node = PlanNode::new(
        Operation::Scan {
            dataset: summary_use.summary.clone(),
            row_identity: None,
        },
        OutputSchema::new(summary_use.columns.clone()),
        summary_use.origin.clone(),
    );
    if let Some(policy) = shape.policy_filter {
        node = filter(node, mapping.expression(policy.value), policy.node.origin());
    }
    for semi_join in &shape.semi_joins {
        node = mapping.semi_join(node, semi_join.node)?;
    }
    for join in &shape.joins {
        node = mapping.join(node, join.node)?;
    }
    if let Some(query_filter) = shape.query_filter {
        node = filter(
            node,
            mapping.expression(query_filter.value),
            query_filter.node.origin(),
        );
    }
    if shape.project.is_some() {
        node = mapping.project(node, &shape)?;
    }
    let keys = shape
        .keys
        .iter()
        .map(|key| NamedExpression {
            expression: mapping.expression(&key.expression),
            ..key.clone()
        })
        .collect();
    Some(PlanNode::new(
        Operation::Aggregate {
            input: Box::new(node),
            keys,
            components: summary_use.components.clone(),
        },
        shape.aggregate.schema().clone(),
        shape.aggregate.origin().clone(),
    ))
}

/// `Filter(input, predicate)`, with the schema of `input`.
fn filter(input: PlanNode, predicate: Expression, origin: &Origin) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Filter {
            input: Box::new(input),
            predicate,
        },
        schema,
        origin.clone(),
    )
}

/// Maps the columns of the home `Scan` to the summary fields of their dimensions.
struct Mapping<'input> {
    summary_use: &'input SummaryUse,
}

impl Mapping<'_> {
    fn column(&self, column: &str) -> Option<ColumnName> {
        self.summary_use
            .renames
            .iter()
            .find(|(base, _)| base.as_str() == column)
            .map(|(_, field)| field.clone())
    }

    fn expression(&self, expression: &Expression) -> Expression {
        renamed(expression, &|column| self.column(column))
    }

    fn conditions(&self, on: &[JoinCondition]) -> Vec<JoinCondition> {
        on.iter()
            .map(|condition| JoinCondition {
                left: self
                    .column(condition.left.as_str())
                    .unwrap_or_else(|| condition.left.clone()),
                right: condition.right.clone(),
            })
            .collect()
    }

    /// The propagation `SemiJoin` `base` over `input`, its keys mapped.
    fn semi_join(&self, input: PlanNode, base: &PlanNode) -> Option<PlanNode> {
        let Operation::SemiJoin {
            relationship,
            on,
            related_all,
            related_visible,
            ..
        } = base.operation()
        else {
            return None;
        };
        let schema = input.schema().clone();
        Some(PlanNode::new(
            Operation::SemiJoin {
                input: Box::new(input),
                relationship: relationship.clone(),
                on: self.conditions(on),
                related_all: related_all.clone(),
                related_visible: related_visible.clone(),
            },
            schema,
            base.origin().clone(),
        ))
    }

    /// The `Join` `base` with `left` as its left input, its `from_columns` mapped.
    fn join(&self, left: PlanNode, base: &PlanNode) -> Option<PlanNode> {
        let Operation::Join {
            right,
            relationship,
            kind,
            on,
            right_prefix,
            ..
        } = base.operation()
        else {
            return None;
        };
        let mut columns = left.schema().columns().to_vec();
        columns.extend(right.schema().columns().iter().cloned());
        Some(PlanNode::new(
            Operation::Join {
                left: Box::new(left),
                right: right.clone(),
                relationship: relationship.clone(),
                kind: *kind,
                on: self.conditions(on),
                right_prefix: right_prefix.clone(),
            },
            OutputSchema::new(columns),
            base.origin().clone(),
        ))
    }

    /// The `Project` of the branch over `input`: every column of `input`, then the computed
    /// dimensions, mapped, with their base columns.
    fn project(&self, input: PlanNode, shape: &BranchShape<'_>) -> Option<PlanNode> {
        let project = shape.project?;
        let computed = shape.computed_dimensions();
        let mut expressions = Vec::new();
        let mut columns = input.schema().columns().to_vec();
        for column in input.schema().columns() {
            expressions.push(NamedExpression {
                name: column.name.clone(),
                expression: column_reference(column.name.as_str())?,
                derived: None,
            });
        }
        for dimension in computed {
            let column = project
                .node
                .schema()
                .columns()
                .iter()
                .find(|column| column.name == dimension.name)?;
            columns.push(column.clone());
            expressions.push(NamedExpression {
                expression: self.expression(&dimension.expression),
                ..dimension.clone()
            });
        }
        Some(PlanNode::new(
            Operation::Project {
                input: Box::new(input),
                expressions,
            },
            OutputSchema::new(columns),
            project.node.origin().clone(),
        ))
    }
}
