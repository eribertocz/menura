//! Unit tests of the `Snapshot` node (E4-3.13).

use std::num::NonZeroU64;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::{AggregateCall, Expression, SimpleAggregate};
use menura_model::internal_model::{
    NullHandling, Selection, SnapshotAggregation, StaleHandling, UnexpectedHandling,
};

use super::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn,
    SnapshotMissing, SnapshotOver, SnapshotRule, TimeSnapshot, UnitKey,
};
use crate::names::ColumnName;
use crate::origin::Origin;
use crate::test_support::{at, in_model, name, reference};

fn column(text: &str, logical_type: LogicalType) -> SchemaColumn {
    SchemaColumn {
        name: ColumnName::new(text),
        logical_type,
        nullable: true,
    }
}

fn node(operation: Operation, columns: &[&str]) -> PlanNode {
    let schema = columns
        .iter()
        .map(|text| column(text, LogicalType::Date))
        .collect();
    PlanNode::new(
        operation,
        OutputSchema::new(schema),
        in_model("/semantic_model/0/datasets/0"),
    )
}

fn scan(dataset: &str) -> PlanNode {
    node(
        Operation::Scan {
            dataset: name(dataset),
            row_identity: None,
        },
        &["customer_id", "period", "balance"],
    )
}

fn field(dataset: &str, field: &str) -> Expression {
    Expression::Reference(reference(dataset, field))
}

fn named(text: &str, expression: Expression) -> NamedExpression {
    NamedExpression {
        name: ColumnName::new(text),
        expression,
        derived: None,
    }
}

fn rule(over: SnapshotOver, aggregation: SnapshotAggregation) -> SnapshotRule {
    SnapshotRule {
        metric: name("BALANCE"),
        keys: vec![named("period", field("debt", "period"))],
        entity: vec![named("customer_id", field("debt", "customer_id"))],
        components: vec![AggregateComponent {
            name: ColumnName::new("balance"),
            call: AggregateCall::Simple {
                function: SimpleAggregate::Sum,
                argument: Box::new(field("debt", "balance")),
            },
            filter: None,
        }],
        aggregation,
        nulls: NullHandling::Keep,
        over,
    }
}

fn time_over(missing: SnapshotMissing) -> SnapshotOver {
    SnapshotOver::Time(TimeSnapshot {
        period: field("debt", "period"),
        grain: Grain::Month,
        unit: Some(UnitKey {
            column: ColumnName::new("period"),
            grain: Grain::Year,
        }),
        expected: Box::new(scan("EXPECTED")),
        missing,
        unexpected: UnexpectedHandling::Exclude,
    })
}

fn snapshot(rule: SnapshotRule) -> PlanNode {
    PlanNode::new(
        Operation::Snapshot {
            input: Box::new(scan("DEBT")),
            rule,
        },
        OutputSchema::new(vec![
            column("period", LogicalType::Date),
            column("customer_id", LogicalType::Text),
            column("balance", LogicalType::Text),
        ]),
        Origin::Query(at("/query/metrics/0")),
    )
}

fn scanned(node: &PlanNode) -> Vec<String> {
    node.inputs()
        .iter()
        .filter_map(|input| match input.operation() {
            Operation::Scan { dataset, .. } => Some(dataset.as_str().to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
fn e4_3_13_snapshot_reads_input_then_expected_then_carried_rows() {
    let carried = SnapshotMissing::CarryForward {
        carried: Box::new(scan("CARRIED")),
        max_carry_forward: NonZeroU64::MIN,
        on_stale: StaleHandling::Error,
    };
    let last = SnapshotAggregation::Last(Selection::GlobalPeriod);

    let node = snapshot(rule(time_over(carried), last));

    assert_eq!(scanned(&node), ["DEBT", "EXPECTED", "CARRIED"]);
}

#[test]
fn e4_3_13_missing_zero_reads_its_added_rows() {
    let zero = SnapshotMissing::Zero {
        added: Box::new(scan("ADDED")),
    };

    let node = snapshot(rule(time_over(zero), SnapshotAggregation::Average));

    assert_eq!(scanned(&node), ["DEBT", "EXPECTED", "ADDED"]);
}

#[test]
fn e4_3_13_a_non_time_over_reads_only_its_input() {
    let over = SnapshotOver::Field(field("stock", "warehouse"));

    let node = snapshot(rule(over, SnapshotAggregation::Maximum));

    assert_eq!(scanned(&node), ["DEBT"]);
    assert!(matches!(
        node.operation(),
        Operation::Snapshot { rule, .. } if rule.metric == name("BALANCE")
    ));
}
