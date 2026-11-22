//! Whether a summary can be used for a branch (SUM-3.1 to SUM-3.6, SUM-3.8), without the
//! catalog state: freshness (SUM-3.7) is checked apart.

use menura_model::internal_model::{Dataset, NormalizedName, Summary, SummaryDimension, ValueType};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{AggregateComponent, PlanNode, SchemaColumn};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::branch::BranchShape;
use super::calendar::{derives, is_aligned};
use super::condition::SummaryCondition;
use super::equivalence::{Recomposed, recompose};
use super::exclusions::{depends_on_build_time, excluded};
use super::reads::{Read, ReadKind, branch_reads};
use crate::context::RewriteContext;

/// What the summary branch of an eligible summary reads (E5-3.5a).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SummaryUse {
    /// The summary dataset.
    pub(super) summary: NormalizedName,
    /// Where the summary dataset is declared: the origin of its `Scan`.
    pub(super) origin: Origin,
    /// The columns of the `Scan` of the summary, in the order of its fields.
    pub(super) columns: Vec<SchemaColumn>,
    /// Each column of the home `Scan` the branch reads, with the summary field of the
    /// dimension whose `base` it is.
    pub(super) renames: Vec<(ColumnName, ColumnName)>,
    /// The recomposed components, in the order of the branch's components (SUM-3.1).
    pub(super) components: Vec<AggregateComponent>,
}

impl SummaryUse {
    /// The summary dataset this use reads.
    #[must_use]
    pub(crate) fn summary(&self) -> &NormalizedName {
        &self.summary
    }
}

/// Whether a summary can be used for a branch, without the catalog state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Eligibility {
    /// Every condition holds; the summary branch reads what the value says.
    Eligible(SummaryUse),
    /// The first condition that fails, in the order of SUM-3.
    Ineligible(SummaryCondition),
}

/// Whether `summary` can be used for `branch`, the plan of one branch ending with its
/// `Aggregate`: [`Eligibility::Eligible`] when SUM-3.1 to SUM-3.6 and SUM-3.8 hold, else the
/// first of them that fails, checked in the order SUM-3.1, SUM-3.2, SUM-3.2a, SUM-3.3,
/// SUM-3.3a, SUM-3.4, SUM-3.5, SUM-3.6, SUM-3.8. Freshness (SUM-3.7) needs the catalog
/// state and is checked apart.
///
/// A summary of another dataset than the branch's home dataset has no measure of its
/// components (SUM-3.1). A branch that is not an `Aggregate` over the nodes of E4-4, such as
/// a semi-additive branch, is excluded (SUM-3.2).
pub(crate) fn eligibility(
    branch: &PlanNode,
    summary: &Summary,
    context: &RewriteContext<'_>,
) -> Eligibility {
    if summary.dataset.as_str().is_empty() {
        return Eligibility::Ineligible(SummaryCondition::Equivalence);
    }
    Eligibility::Ineligible(SummaryCondition::Equivalence)
}

fn unused_entry(
    branch: &PlanNode,
    summary: &Summary,
    context: &RewriteContext<'_>,
) -> Eligibility {
    match summary_use(branch, summary, context) {
        Ok(summary_use) => Eligibility::Eligible(summary_use),
        Err(condition) => Eligibility::Ineligible(condition),
    }
}

fn summary_use(
    branch: &PlanNode,
    summary: &Summary,
    context: &RewriteContext<'_>,
) -> Result<SummaryUse, SummaryCondition> {
    let Some(shape) = BranchShape::of(branch) else {
        return Err(SummaryCondition::Exclusions);
    };
    if shape.home != &summary.of {
        return Err(SummaryCondition::Equivalence);
    }
    let recomposed = recompose(shape.components, summary).ok_or(SummaryCondition::Equivalence)?;
    if excluded(&shape, context.model, summary) {
        return Err(SummaryCondition::Exclusions);
    }
    let base = context
        .model
        .dataset(&summary.of)
        .ok_or(SummaryCondition::QueryDependentValues)?;
    if depends_on_build_time(summary, base, context.schema) {
        return Err(SummaryCondition::QueryDependentValues);
    }
    let reads = branch_reads(&shape, context.query);
    grain(&reads, summary, context.query)?;
    let dataset = context
        .model
        .dataset(&summary.dataset)
        .ok_or(SummaryCondition::PolicyPreservation)?;
    policies(&reads, summary, dataset, context.query)?;
    used(summary, dataset, &reads, recomposed)
}

/// The dimension of `summary` whose `base` is the field of the home `Scan` column `column`.
fn dimension<'summary>(
    summary: &'summary Summary,
    column: &ColumnName,
) -> Option<&'summary SummaryDimension> {
    summary
        .dimensions
        .iter()
        .find(|dimension| dimension.base.as_str() == column.as_str())
}

/// SUM-3.3, SUM-3.3a, SUM-3.4 and SUM-3.5, in that order.
fn grain(reads: &[Read], summary: &Summary, query: &BoundQuery) -> Result<(), SummaryCondition> {
    let queried = || reads.iter().filter(|read| read.kind != ReadKind::Policy);
    if queried().any(|read| dimension(summary, &read.column).is_none()) {
        return Err(SummaryCondition::Grain);
    }
    let grain_of = |read: &Read| dimension(summary, &read.column).and_then(|found| found.grain);
    let truncated = reads.iter().any(|read| {
        matches!(read.kind, ReadKind::Plain | ReadKind::Policy) && grain_of(read).is_some()
    });
    if truncated {
        return Err(SummaryCondition::TruncatedFields);
    }
    let underived = reads.iter().any(|read| match (read.kind, grain_of(read)) {
        (ReadKind::Truncated(unit), Some(grain)) => !derives(grain, unit),
        _ => false,
    });
    if underived {
        return Err(SummaryCondition::TimeGrain);
    }
    let unaligned = query.date_range().is_some_and(|range| {
        reads.iter().any(|read| match (read.kind, grain_of(read)) {
            (ReadKind::DateRange, Some(grain)) => !is_aligned(range.from, range.to, grain),
            _ => false,
        })
    });
    if unaligned {
        return Err(SummaryCondition::TimeFilterAlignment);
    }
    Ok(())
}

/// SUM-3.6: the user has access to the base, the summary declares no policies of its own, and
/// every column the base's policies read, its own conditions and its propagation keys, is the
/// `base` of a summary dimension.
fn policies(
    reads: &[Read],
    summary: &Summary,
    dataset: &Dataset,
    query: &BoundQuery,
) -> Result<(), SummaryCondition> {
    let kept = reads
        .iter()
        .filter(|read| read.kind == ReadKind::Policy)
        .all(|read| dimension(summary, &read.column).is_some());
    if kept && dataset.row_policies.is_empty() && query.subgraph().contains_dataset(&summary.of) {
        Ok(())
    } else {
        Err(SummaryCondition::PolicyPreservation)
    }
}

/// What the summary branch reads; SUM-3.8 fails when a summary field it reads has no logical
/// type.
fn used(
    summary: &Summary,
    dataset: &Dataset,
    reads: &[Read],
    recomposed: Vec<Recomposed>,
) -> Result<SummaryUse, SummaryCondition> {
    let mut renames: Vec<(ColumnName, ColumnName)> = Vec::new();
    for read in reads {
        let mapped = dimension(summary, &read.column)
            .map(|found| ColumnName::new(found.field.as_str()))
            .ok_or(SummaryCondition::PolicyPreservation)?;
        if !renames.iter().any(|(column, _)| column == &read.column) {
            renames.push((read.column.clone(), mapped));
        }
    }
    let read_fields: Vec<&str> = renames
        .iter()
        .map(|(_, field)| field.as_str())
        .chain(
            recomposed
                .iter()
                .flat_map(|one| one.reads.iter().map(NormalizedName::as_str)),
        )
        .collect();
    let mut columns = Vec::new();
    for field in &dataset.fields {
        if !read_fields.contains(&field.name.as_str()) {
            continue;
        }
        let Some(typed) = &field.typed else {
            return Err(SummaryCondition::SameType);
        };
        let ValueType::Logical(logical_type) = &typed.value_type else {
            return Err(SummaryCondition::SameType);
        };
        columns.push(SchemaColumn {
            name: ColumnName::new(field.name.as_str()),
            logical_type: logical_type.clone(),
            nullable: typed.nullable,
        });
    }
    if columns.len()
        != read_fields
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
    {
        return Err(SummaryCondition::SameType);
    }
    Ok(SummaryUse {
        summary: summary.dataset.clone(),
        origin: Origin::Model(dataset.location.clone()),
        columns,
        renames,
        components: recomposed.into_iter().map(|one| one.component).collect(),
    })
}
