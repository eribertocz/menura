//! Unit tests of the logical plan (E4-3).

use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::{AggregateCall, Expression, Identifier, Reference, SimpleAggregate};

use super::{
    AggregateComponent, JoinKind, NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn,
    SortItem,
};
use crate::names::ColumnName;
use crate::origin::Origin;
use crate::paths::JoinStep;
use crate::test_support::{at, in_model, name, reference, true_literal};

fn column(text: &str, logical_type: LogicalType, nullable: bool) -> SchemaColumn {
    SchemaColumn {
        name: ColumnName::new(text),
        logical_type,
        nullable,
    }
}

fn count_type() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::SixtyFour,
        signed: true,
    }
}

fn region_schema() -> OutputSchema {
    OutputSchema::new(vec![column("region", LogicalType::Text, true)])
}

fn query_origin() -> Origin {
    Origin::Query(at("/query"))
}

fn scan(dataset: &str) -> PlanNode {
    PlanNode::new(
        Operation::Scan {
            dataset: name(dataset),
        },
        region_schema(),
        in_model("/semantic_model/0/datasets/0"),
    )
}

fn field(dataset: &str, field: &str) -> Expression {
    Expression::Reference(reference(dataset, field))
}

fn kind_of(operation: &Operation) -> &'static str {
    match operation {
        Operation::Scan { .. } => "Scan",
        Operation::Join { .. } => "Join",
        Operation::Filter { .. } => "Filter",
        Operation::SemiJoin { .. } => "SemiJoin",
        Operation::Deduplicate { .. } => "Deduplicate",
        Operation::Aggregate { .. } => "Aggregate",
        Operation::Stitch { .. } => "Stitch",
        Operation::Project { .. } => "Project",
        Operation::Sort { .. } => "Sort",
        Operation::Limit { .. } => "Limit",
    }
}

fn aggregate_schema() -> OutputSchema {
    OutputSchema::new(vec![
        column("region", LogicalType::Text, true),
        column("order_count", count_type(), false),
    ])
}

fn filtered_count() -> PlanNode {
    let filter = PlanNode::new(
        Operation::Filter {
            input: Box::new(scan("ORDERS")),
            predicate: true_literal(),
        },
        region_schema(),
        Origin::Query(at("/query/filter")),
    );
    PlanNode::new(
        Operation::Aggregate {
            input: Box::new(filter),
            keys: vec![NamedExpression {
                name: ColumnName::new("region"),
                expression: field("orders", "region"),
            }],
            components: vec![AggregateComponent {
                name: ColumnName::new("order_count"),
                call: AggregateCall::CountRows,
                filter: None,
            }],
        },
        aggregate_schema(),
        query_origin(),
    )
}

fn version_0_3_0_chain() -> PlanNode {
    let project = PlanNode::new(
        Operation::Project {
            input: Box::new(filtered_count()),
            expressions: vec![NamedExpression {
                name: ColumnName::new("order_count"),
                expression: Expression::Reference(Reference {
                    qualifier: None,
                    name: Identifier::regular("order_count").expect("a regular identifier"),
                }),
            }],
        },
        aggregate_schema(),
        query_origin(),
    );
    let sort = PlanNode::new(
        Operation::Sort {
            input: Box::new(project),
            items: vec![SortItem {
                column: ColumnName::new("order_count"),
                direction: Direction::Desc,
                nulls: NullsOrder::Last,
            }],
        },
        aggregate_schema(),
        Origin::Query(at("/query/nonportable_extensions/0/data/order_by")),
    );
    PlanNode::new(
        Operation::Limit {
            input: Box::new(sort),
            rows: 10,
        },
        aggregate_schema(),
        Origin::Query(at("/query/nonportable_extensions/0/data/limit")),
    )
}

#[test]
fn e4_3_every_node_carries_its_output_schema_and_origin() {
    let limit = version_0_3_0_chain();

    let mut kinds = Vec::new();
    let mut node = Some(&limit);
    while let Some(current) = node {
        assert!(!current.schema().columns().is_empty());
        kinds.push(kind_of(current.operation()));
        node = current.inputs().first().copied();
    }
    assert_eq!(
        kinds,
        ["Limit", "Sort", "Project", "Aggregate", "Filter", "Scan"]
    );
    assert_eq!(
        limit.schema().columns().get(1),
        Some(&column("order_count", count_type(), false))
    );
    assert_eq!(
        limit.origin(),
        &Origin::Query(at("/query/nonportable_extensions/0/data/limit"))
    );
}

#[test]
fn e4_3_join_and_semi_join_keep_their_relationships() {
    let join = PlanNode::new(
        Operation::Join {
            left: Box::new(scan("ORDERS")),
            right: Box::new(scan("CUSTOMERS")),
            relationship: name("ORDERS_TO_CUSTOMERS"),
            kind: JoinKind::Left,
        },
        region_schema(),
        query_origin(),
    );
    let semi_join = PlanNode::new(
        Operation::SemiJoin {
            input: Box::new(join),
            path: vec![JoinStep {
                relationship: name("ORDERS_TO_CUSTOMERS"),
                from: name("ORDERS"),
                to: name("CUSTOMERS"),
            }],
            predicate: true_literal(),
        },
        region_schema(),
        in_model("/semantic_model/0/datasets/1/custom_extensions/0"),
    );

    assert_eq!(semi_join.inputs().len(), 1);
    let join = semi_join.inputs().first().copied().expect("one input");
    assert_eq!(join.inputs().len(), 2);
    assert!(matches!(
        join.operation(),
        Operation::Join {
            kind: JoinKind::Left,
            ..
        }
    ));
}

#[test]
fn e4_3_deduplicate_and_stitch_keep_their_keys() {
    let deduplicate = PlanNode::new(
        Operation::Deduplicate {
            input: Box::new(scan("ORDERS")),
            group: vec![ColumnName::new("region")],
            identity: vec![ColumnName::new("order_id")],
        },
        region_schema(),
        query_origin(),
    );
    let aggregate = |input: PlanNode| {
        PlanNode::new(
            Operation::Aggregate {
                input: Box::new(input),
                keys: Vec::new(),
                components: vec![AggregateComponent {
                    name: ColumnName::new("total"),
                    call: AggregateCall::Simple {
                        function: SimpleAggregate::Sum,
                        argument: Box::new(field("orders", "amount")),
                    },
                    filter: Some(true_literal()),
                }],
            },
            region_schema(),
            query_origin(),
        )
    };
    let stitch = PlanNode::new(
        Operation::Stitch {
            branches: vec![aggregate(deduplicate), aggregate(scan("RETURNS"))],
            keys: vec![ColumnName::new("region")],
        },
        region_schema(),
        query_origin(),
    );

    assert_eq!(stitch.inputs().len(), 2);
    assert!(matches!(
        stitch.operation(),
        Operation::Stitch { keys, .. } if keys == &[ColumnName::new("region")]
    ));
    let first_branch = stitch.inputs().first().copied().expect("two branches");
    let deduplicate = first_branch.inputs().first().copied().expect("one input");
    assert_eq!(kind_of(deduplicate.operation()), "Deduplicate");
}
