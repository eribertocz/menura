//! The lineage of every node of a logical plan, computed from its inputs (E4-3).

use menura_model::expression::Expression;
use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{
    AggregateComponent, JoinCondition, LookupColumn, NamedExpression, Operation, PlanNode,
    StitchColumn, TemporalCoverage,
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
            Operation::Filter { input, .. } | Operation::ShiftedFilter { input, .. } => {
                let mut lineage = self.node(input, prefix)?;
                let read = lineage.provenance_of(filter_columns(node.operation()));
                lineage.read_in_every_branch(&read);
                Ok(lineage)
            }
            Operation::SemiJoin { .. } => self.semi_join(node, prefix),
            Operation::ToDate { .. } => self.repeated_to_date(node, prefix),
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
            Operation::Lookup { .. } => self.lookup(node),
            Operation::Population { .. } | Operation::Snapshot { .. } => self.semi_additive(node),
            Operation::Periods { .. } => Ok(reads_nothing(node)),
            Operation::PeriodRange { .. } | Operation::FullJoin { .. } => Err(InternalError::new(
                "core",
                "a compiled plan holds an operation of the integrity checks",
            )),
        }
    }

    /// The lineage of the `ToDate` `node`: its period comes from the time value it reads
    /// (E4-3.15).
    fn repeated_to_date(
        &self,
        node: &PlanNode,
        prefix: Option<&ColumnPrefix>,
    ) -> Result<NodeLineage, InternalError> {
        let Operation::ToDate {
            input,
            dimension,
            period,
            ..
        } = node.operation()
        else {
            return Err(InternalError::new(
                "core",
                "a to-date lineage over another node",
            ));
        };
        let mut lineage = self.node(input, prefix)?;
        let read = lineage.provenance_of([dimension.as_str()]);
        lineage.read_in_every_branch(&read);
        lineage.columns.insert(period.as_str().to_owned(), read);
        Ok(lineage)
    }

    /// The lineage of the `Lookup` `node`.
    fn lookup(&self, node: &PlanNode) -> Result<NodeLineage, InternalError> {
        let Operation::Lookup {
            result,
            value,
            columns,
            ..
        } = node.operation()
        else {
            return Err(InternalError::new(
                "core",
                "a lookup lineage over another node",
            ));
        };
        Ok(lookup(
            self.node(result, None)?,
            self.node(value, None)?,
            columns,
        ))
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
            temporal,
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
        Ok(join(left, right, relationship, on, temporal.as_ref()))
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
            temporal,
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
        Ok(semi_join(input, all, visible, on, temporal.as_ref()))
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
/// `on` (PLN-4.4), and, along a temporal relationship, from the `from_time`, `valid_from` and
/// `valid_to` fields of its coverage condition (VAL-2.24). The conditions are read in every
/// branch (PLN-4.5).
fn join(
    mut left: NodeLineage,
    mut right: NodeLineage,
    relationship: &NormalizedName,
    on: &[JoinCondition],
    temporal: Option<&TemporalCoverage>,
) -> NodeLineage {
    let mut left_keys = left.provenance_of(on.iter().map(|pair| pair.left.as_str()));
    let mut right_keys = right.provenance_of(on.iter().map(|pair| pair.right.as_str()));
    let mut coverage = Provenance::default();
    if let Some(temporal) = temporal {
        let time = left.provenance_of([temporal.from_time.as_str()]);
        let range = right.provenance_of(range_columns(temporal));
        left_keys.absorb(&time);
        right_keys.absorb(&range);
        coverage.absorb(&time);
        coverage.absorb(&range);
    }
    for provenance in right.columns.values_mut() {
        provenance.relationships.insert(relationship.clone());
        provenance
            .relationships
            .extend(left_keys.relationships.iter().cloned());
        provenance.fields.extend(coverage.fields.iter().cloned());
    }
    left.read_in_every_branch(&left_keys);
    left.read_in_every_branch(&right_keys);
    let columns = std::mem::take(&mut right.columns);
    left.columns.extend(columns);
    left.absorb_reads(right);
    left
}

/// `SemiJoin`: the columns of `input`; the keys of `on` on both sides, the columns of the
/// coverage condition of a temporal relationship, and everything the plans of the related rows
/// read, are read in every branch (PLN-4.5).
fn semi_join(
    mut input: NodeLineage,
    all: NodeLineage,
    visible: NodeLineage,
    on: &[JoinCondition],
    temporal: Option<&TemporalCoverage>,
) -> NodeLineage {
    let mut input_keys = input.provenance_of(on.iter().map(|pair| pair.left.as_str()));
    if let Some(temporal) = temporal {
        input_keys.absorb(&input.provenance_of([temporal.from_time.as_str()]));
    }
    input.read_in_every_branch(&input_keys);
    for related in [all, visible] {
        let mut related_keys = related.provenance_of(on.iter().map(|pair| pair.right.as_str()));
        if let Some(temporal) = temporal {
            related_keys.absorb(&related.provenance_of(range_columns(temporal)));
        }
        input.read_in_every_branch(&related_keys);
        input.absorb_reads(related);
    }
    input
}

/// The `valid_from` and `valid_to` columns of the coverage condition `temporal`.
fn range_columns(temporal: &TemporalCoverage) -> [&str; 2] {
    [
        temporal.range.valid_from.as_str(),
        temporal.range.valid_to.as_str(),
    ]
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

/// `Lookup(result, value)`: the columns of `result`, then each attached column from the
/// context column it copies. The branches of the context query follow those of the result
/// rows (E4-8.8), so the branches of `value` are numbered after those of `result` (PLN-4.8).
fn lookup(result: NodeLineage, value: NodeLineage, columns: &[LookupColumn]) -> NodeLineage {
    let mut lineage = with_branches(result);
    let value = with_branches(value);
    let offset = lineage.branches.len();
    for column in columns {
        let mut provenance = value.provenance_of([column.value.as_str()]);
        provenance.branches = provenance
            .branches
            .iter()
            .map(|branch| branch + offset)
            .collect();
        lineage
            .columns
            .insert(column.name.as_str().to_owned(), provenance);
    }
    lineage.branches.extend(value.branches);
    lineage
}

/// `lineage` with its branches explicit: a plan part without a `Stitch` is its one branch,
/// which reads what the part reads in every branch, and every column comes from it.
fn with_branches(mut lineage: NodeLineage) -> NodeLineage {
    if lineage.branches.is_empty() {
        lineage.branches.push(std::mem::take(&mut lineage.shared));
        for provenance in lineage.columns.values_mut() {
            provenance.branches.insert(0);
        }
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

/// The columns a `Filter` or a `ShiftedFilter` reads: its predicate, and the column of its
/// shifted range (E4-8.4); none for another operation.
fn filter_columns(operation: &Operation) -> Vec<&str> {
    match operation {
        Operation::Filter { predicate, .. } => columns_read(predicate),
        Operation::ShiftedFilter {
            predicate, range, ..
        } => {
            let mut columns = columns_read(&range.column);
            if let Some(predicate) = predicate {
                columns.extend(columns_read(predicate));
            }
            columns
        }
        _ => Vec::new(),
    }
}
