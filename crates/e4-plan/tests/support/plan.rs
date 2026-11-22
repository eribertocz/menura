//! Helpers that read a logical plan.

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, Identifier, Reference};
use menura_representations::logical_plan::{Operation, PlanNode, SchemaColumn};
use menura_representations::names::ColumnName;

/// The name of the operation of `node`.
pub fn kind(node: &PlanNode) -> &'static str {
    match node.operation() {
        Operation::Scan { .. } => "Scan",
        Operation::Join { .. } => "Join",
        Operation::Filter { .. } => "Filter",
        Operation::ShiftedFilter { .. } => "ShiftedFilter",
        Operation::SemiJoin { .. } => "SemiJoin",
        Operation::Deduplicate { .. } => "Deduplicate",
        Operation::Aggregate { .. } => "Aggregate",
        Operation::Population { .. } => "Population",
        Operation::Periods { .. } => "Periods",
        Operation::Snapshot { .. } => "Snapshot",
        Operation::Stitch { .. } => "Stitch",
        Operation::Lookup { .. } => "Lookup",
        Operation::ToDate { .. } => "ToDate",
        Operation::Project { .. } => "Project",
        Operation::Sort { .. } => "Sort",
        Operation::Limit { .. } => "Limit",
        Operation::PeriodRange { .. } => "PeriodRange",
        Operation::FullJoin { .. } => "FullJoin",
    }
}

/// The nodes from `plan` down to its `Scan`, following the first input.
pub fn chain(plan: &PlanNode) -> Vec<&PlanNode> {
    let mut nodes = Vec::new();
    let mut node = Some(plan);
    while let Some(current) = node {
        nodes.push(current);
        node = current.inputs().first().copied();
    }
    nodes
}

/// The operation names of [`chain`].
pub fn kinds(plan: &PlanNode) -> Vec<&'static str> {
    chain(plan).into_iter().map(kind).collect()
}

/// The first node of `kind` in [`chain`].
pub fn node<'plan>(plan: &'plan PlanNode, wanted: &str) -> &'plan PlanNode {
    chain(plan)
        .into_iter()
        .find(|candidate| kind(candidate) == wanted)
        .expect("a node of that kind")
}

/// Every node of `kind` in [`chain`], from the top.
pub fn nodes<'plan>(plan: &'plan PlanNode, wanted: &str) -> Vec<&'plan PlanNode> {
    chain(plan)
        .into_iter()
        .filter(|candidate| kind(candidate) == wanted)
        .collect()
}

/// The predicate of a `Filter`.
pub fn predicate(filter: &PlanNode) -> &Expression {
    match filter.operation() {
        Operation::Filter { predicate, .. } => predicate,
        _ => panic!("not a Filter"),
    }
}

/// The names of the schema of `node`.
pub fn column_names(node: &PlanNode) -> Vec<&str> {
    node.schema()
        .columns()
        .iter()
        .map(|column| column.name.as_str())
        .collect()
}

/// A schema column.
pub fn column(name: &str, logical_type: LogicalType, nullable: bool) -> SchemaColumn {
    SchemaColumn {
        name: ColumnName::new(name),
        logical_type,
        nullable,
    }
}

/// The unqualified reference to the column `name`.
pub fn column_reference(name: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::delimited(name).expect("a column name"),
    })
}
