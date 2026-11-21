//! Plans of the join tests, and helpers that read their `Join`, `Aggregate` and
//! `Deduplicate` nodes.

use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_model::expression::Expression;
use menura_representations::bound_query::{BoundDimension, BoundQueryParts};
use menura_representations::logical_plan::{JoinCondition, JoinKind, Operation, PlanNode};
use menura_representations::names::ColumnName;
use menura_representations::paths::{Branches, QueryElement};

use super::joins::{joined_branches, joined_dimension, path, sales_model, to_customers};
use super::plan::node;
use super::{anyone, build_plan, metric_reference, query};

/// The name of the synthesized row identity when no field takes it.
pub const IDENTITY: &str = "menura.row_identity";

/// The plan over [`sales_model`] of `dimensions`, `metric_count` times `TOTAL_AMOUNT`, and
/// `parts`.
pub fn plan(
    branches: &Branches,
    dimensions: Vec<BoundDimension>,
    metric_count: usize,
    parts: BoundQueryParts,
) -> PlanNode {
    let metrics = (0..metric_count)
        .map(|index| metric_reference("TOTAL_AMOUNT", index))
        .collect();
    build_plan(
        branches,
        &query(dimensions, metrics, parts),
        &anyone(),
        &sales_model(),
    )
    .expect("a plan")
}

/// The plan of `CUSTOMERS.SEGMENT` and `TOTAL_AMOUNT`, through `ORDERS_TO_CUSTOMERS`.
pub fn segment_plan() -> PlanNode {
    plan(
        &joined_branches(
            1,
            vec![path(QueryElement::Dimension(0), vec![to_customers()])],
        ),
        vec![joined_dimension("CUSTOMERS", "SEGMENT", None, 0)],
        1,
        BoundQueryParts::default(),
    )
}

/// The left input, right input, conditions, right prefix and kind of a `Join`.
pub fn join_parts(join: &PlanNode) -> (&PlanNode, &PlanNode, &[JoinCondition], &str, JoinKind) {
    match join.operation() {
        Operation::Join {
            left,
            right,
            on,
            right_prefix,
            kind,
            ..
        } => (left, right, on, right_prefix.as_str(), *kind),
        _ => panic!("not a Join"),
    }
}

/// The condition `left = right`.
pub fn condition(left: &str, right: &str) -> JoinCondition {
    JoinCondition {
        left: ColumnName::new(left),
        right: ColumnName::new(right),
    }
}

/// The key expressions of the first `Aggregate` of `plan`.
pub fn aggregate_keys(plan: &PlanNode) -> Vec<Expression> {
    match node(plan, "Aggregate").operation() {
        Operation::Aggregate { keys, .. } => {
            keys.iter().map(|key| key.expression.clone()).collect()
        }
        _ => panic!("not an Aggregate"),
    }
}

/// The group and identity of the first `Deduplicate` of `plan`.
pub fn deduplicate_parts(plan: &PlanNode) -> (Vec<Expression>, Vec<&str>) {
    match node(plan, "Deduplicate").operation() {
        Operation::Deduplicate {
            group, identity, ..
        } => (
            group.clone(),
            identity.iter().map(ColumnName::as_str).collect(),
        ),
        _ => panic!("not a Deduplicate"),
    }
}

/// The type of the row identity.
pub fn identity_type() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::SixtyFour,
        signed: true,
    }
}

/// The dataset a `Scan` reads.
pub fn scan_dataset(scan: &PlanNode) -> &str {
    match scan.operation() {
        Operation::Scan { dataset, .. } => dataset.as_str(),
        _ => panic!("not a Scan"),
    }
}

/// The row identity a `Scan` synthesizes, when it does.
pub fn row_identity(scan: &PlanNode) -> Option<&str> {
    match scan.operation() {
        Operation::Scan { row_identity, .. } => row_identity.as_ref().map(ColumnName::as_str),
        _ => panic!("not a Scan"),
    }
}
