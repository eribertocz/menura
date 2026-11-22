//! The lineage of every node of a logical plan, computed from its inputs (E4-3).

use menura_model::expression::Expression;
use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{
    AggregateComponent, JoinCondition, NamedExpression, Operation, PlanNode, StitchColumn,
};
use menura_representations::names::{ColumnName, ColumnPrefix};

use super::provenance::{ColumnProvenance, NodeLineage, Provenance};
use super::references::{columns_read, component_columns};
use super::scan::Sources;
use crate::compile::InternalError;

impl Sources<'_> {
    /// The lineage of `node`, whose `Scan` columns are under `prefix` when it is the right
    /// input of a `Join`, or the input of a `Filter` or `SemiJoin` that is (E6-4.12).
    ///
    /// # Errors
    ///
    /// An [`InternalError`] for an element the plan names and its inputs do not have.
    pub(super) fn node(
        &self,
        node: &PlanNode,
        prefix: Option<&ColumnPrefix>,
    ) -> Result<NodeLineage, InternalError> {
        match node.operation() {
            Operation::Scan {
                dataset,
                row_identity,
            } => self.scan(node, dataset, row_identity.as_ref(), prefix),
            Operation::Join { .. } => self.join(node, prefix),
            Operation::Filter { input, predicate } => {
                let mut lineage = self.node(input, prefix)?;
                let read = lineage.provenance_of(columns_read(predicate));
                lineage.read_in_every_branch(&read);
                Ok(lineage)
            }
            Operation::SemiJoin { .. } => self.semi_join(node, prefix),
            Operation::Deduplicate { input, .. }
            | Operation::Sort { input, .. }
            | Operation::Limit { input, .. } => self.node(input, None),
            Operation::Aggregate {
                input,
                keys,
                components,
            } => Ok(aggregate(self.node(input, None)?, keys, components)),
            Operation::Project { input, expressions } => {
                Ok(project(self.node(input, None)?, expressions))
            }
            Operation::Stitch {
                branches,
                keys,
                columns,
            } => {
                let branches = branches
                    .iter()
                    .map(|branch| self.node(branch, None))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(stitch(branches, keys, columns))
            }
            Operation::Population { .. } | Operation::Snapshot { .. } => self.semi_additive(node),
            Operation::Periods { .. } => Ok(reads_nothing(node)),
        }
    }

    /// The lineage of the `Join` `node`, whose left `Scan` columns are under `prefix`.
    fn join(
        &self,
        node: &PlanNode,
        prefix: Option<&ColumnPrefix>,
    ) -> Result<NodeLineage, InternalError> {
        let Operation::Join {
            left,
            right,
            relationship,
            on,
            right_prefix,
            ..
        } = node.operation()
        else {
            return Err(InternalError::new(
                "core",
                "a join lineage over another node",
            ));
        };
        let left = self.node(left, prefix)?;
        let right = self.node(right, Some(right_prefix))?;
        Ok(join(left, right, relationship, on))
    }

    /// The lineage of the `SemiJoin` `node`, whose input `Scan` columns are under `prefix`.
    fn semi_join(
        &self,
        node: &PlanNode,
        prefix: Option<&ColumnPrefix>,
    ) -> Result<NodeLineage, InternalError> {
        let Operation::SemiJoin {
            input,
            on,
            related_all,
            related_visible,
            ..
        } = node.operation()
        else {
            return Err(InternalError::new(
                "core",
                "a semi-join lineage over another node",
            ));
        };
        let input = self.node(input, prefix)?;
        let all = self.node(related_all, None)?;
        let visible = self.node(related_visible, None)?;
        Ok(semi_join(input, all, visible, on))
    }
}

/// A node whose columns read no column: every column has an empty provenance.
pub(super) fn reads_nothing(node: &PlanNode) -> NodeLineage {
    let mut lineage = NodeLineage::default();
    for column in node.schema().columns() {
        lineage
            .columns
            .insert(column.name.as_str().to_owned(), Provenance::default());
    }
    lineage
}

/// `Join(left, right, relationship)`: the columns of both inputs; each column of `right` also
/// derives from `relationship` and from the relationships that bring the `left` columns of
/// `on` (PLN-4.4). The conditions are read in every branch (PLN-4.5).
fn join(
    mut left: NodeLineage,
    mut right: NodeLineage,
    relationship: &NormalizedName,
    on: &[JoinCondition],
) -> NodeLineage {
    let left_keys = left.provenance_of(on.iter().map(|pair| pair.left.as_str()));
    let right_keys = right.provenance_of(on.iter().map(|pair| pair.right.as_str()));
    for provenance in right.columns.values_mut() {
        provenance.relationships.insert(relationship.clone());
        provenance
            .relationships
            .extend(left_keys.relationships.iter().cloned());
    }
    left.read_in_every_branch(&left_keys);
    left.read_in_every_branch(&right_keys);
    let columns = std::mem::take(&mut right.columns);
    left.columns.extend(columns);
    left.absorb_reads(right);
    left
}

/// `SemiJoin`: the columns of `input`; the keys of `on` on both sides, and everything the
/// plans of the related rows read, are read in every branch (PLN-4.5).
fn semi_join(
    mut input: NodeLineage,
    all: NodeLineage,
    visible: NodeLineage,
    on: &[JoinCondition],
) -> NodeLineage {
    let input_keys = input.provenance_of(on.iter().map(|pair| pair.left.as_str()));
    input.read_in_every_branch(&input_keys);
    for related in [all, visible] {
        let related_keys = related.provenance_of(on.iter().map(|pair| pair.right.as_str()));
        input.read_in_every_branch(&related_keys);
        input.absorb_reads(related);
    }
    input
}

/// `Aggregate`: each key derives from the columns its expression reads, each component from
/// those of its call and filter.
fn aggregate(
    mut input: NodeLineage,
    keys: &[NamedExpression],
    components: &[AggregateComponent],
) -> NodeLineage {
    let mut columns = ColumnProvenance::new();
    for key in keys {
        let read = input.provenance_of(columns_read(&key.expression));
        columns.insert(key.name.as_str().to_owned(), read);
    }
    for component in components {
        let names = component_columns(component);
        columns.insert(
            component.name.as_str().to_owned(),
            input.provenance_of(names),
        );
    }
    input.columns = columns;
    input
}

/// `Project`: each column derives from the columns its expression reads.
fn project(mut input: NodeLineage, expressions: &[NamedExpression]) -> NodeLineage {
    let columns = expressions
        .iter()
        .map(|named| {
            (
                named.name.as_str().to_owned(),
                input.provenance_of(columns_read(&named.expression)),
            )
        })
        .collect();
    input.columns = columns;
    input
}

/// `Stitch`: each key derives from the key of every branch, each component column from the
/// branch that computes it; what each branch reads becomes that branch's reads (PLN-4.8).
fn stitch(
    branches: Vec<NodeLineage>,
    keys: &[ColumnName],
    columns: &[StitchColumn],
) -> NodeLineage {
    let mut lineage = NodeLineage::default();
    for key in keys {
        let mut provenance = Provenance::default();
        for (position, branch) in branches.iter().enumerate() {
            provenance.absorb(&branch.provenance_of([key.as_str()]));
            provenance.branches.insert(position);
        }
        lineage.columns.insert(key.as_str().to_owned(), provenance);
    }
    for column in columns {
        let mut provenance = branches
            .get(column.branch)
            .map(|branch| branch.provenance_of([column.name.as_str()]))
            .unwrap_or_default();
        provenance.branches.insert(column.branch);
        lineage
            .columns
            .insert(column.name.as_str().to_owned(), provenance);
    }
    for branch in branches {
        let mut reads = branch.shared;
        for nested in branch.branches {
            reads.absorb(nested);
        }
        lineage.branches.push(reads);
    }
    lineage
}

/// The provenance of every column `expressions` read in `lineage`.
pub(super) fn read_by<'expression>(
    lineage: &NodeLineage,
    expressions: impl IntoIterator<Item = &'expression Expression>,
) -> Provenance {
    let mut provenance = Provenance::default();
    for expression in expressions {
        provenance.absorb(&lineage.provenance_of(columns_read(expression)));
    }
    provenance
}
