//! Unit tests of the logical plan (E4-3).

use menura_contracts::types::common::{Grain, IntegerBits, LogicalType};
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::{AggregateCall, Expression, Identifier, Reference, SimpleAggregate};
use menura_model::internal_model::PartialPeriod;

use super::{
    AggregateComponent, JoinCondition, JoinKind, LookupColumn, LookupKey, Membership,
    NamedExpression, Operation, OutputSchema, PlanNode, PopulationRule, SchemaColumn, SortItem,
    StitchColumn,
};
use crate::names::{ColumnName, ColumnPrefix};
use crate::origin::Origin;
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

fn column_reference(text: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::delimited(text).expect("a column name"),
    })
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
            row_identity: None,
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
        Operation::ShiftedFilter { .. } => "ShiftedFilter",
        Operation::SemiJoin { .. } => "SemiJoin",
        Operation::Deduplicate { .. } => "Deduplicate",
        Operation::Aggregate { .. } => "Aggregate",
        Operation::Population { .. } => "Population",
        Operation::Periods { .. } => "Periods",
        Operation::Snapshot { .. } => "Snapshot",
        Operation::Stitch { .. } => "Stitch",
        Operation::Lookup { .. } => "Lookup",
        Operation::Project { .. } => "Project",
        Operation::Sort { .. } => "Sort",
        Operation::Limit { .. } => "Limit",
        Operation::PeriodRange { .. } => "PeriodRange",
        Operation::FullJoin { .. } => "FullJoin",
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
                derived: None,
            }],
            components: vec![AggregateComponent {
                name: ColumnName::new("order_count"),
                call: AggregateCall::CountRows,
                filter: None,
                recomposition: None,
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
                derived: None,
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
fn e4_3_2_e4_3_4_join_and_semi_join_keep_their_relationships() {
    let join = PlanNode::new(
        Operation::Join {
            left: Box::new(scan("ORDERS")),
            right: Box::new(scan("CUSTOMERS")),
            relationship: name("ORDERS_TO_CUSTOMERS"),
            kind: JoinKind::Left,
            on: vec![JoinCondition {
                left: ColumnName::new("customer_id"),
                right: ColumnName::new("ORDERS_TO_CUSTOMERS.customer_id"),
            }],
            right_prefix: ColumnPrefix::new("ORDERS_TO_CUSTOMERS."),
        },
        region_schema(),
        query_origin(),
    );
    let semi_join = PlanNode::new(
        Operation::SemiJoin {
            input: Box::new(join),
            relationship: name("CUSTOMERS_TO_REGIONS"),
            on: vec![JoinCondition {
                left: ColumnName::new("ORDERS_TO_CUSTOMERS.region_id"),
                right: ColumnName::new("region_id"),
            }],
            related_all: Box::new(scan("REGIONS")),
            related_visible: Box::new(scan("REGIONS")),
        },
        region_schema(),
        in_model("/semantic_model/0/datasets/1/custom_extensions/0"),
    );

    assert_eq!(semi_join.inputs().len(), 3);
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

/// The `Aggregate` of `input` that sums `orders.amount` with a filter.
fn filtered_total(input: PlanNode) -> PlanNode {
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
                recomposition: None,
            }],
        },
        region_schema(),
        query_origin(),
    )
}

#[test]
fn e4_3_5_e4_3_7_deduplicate_and_stitch_keep_their_keys() {
    let deduplicate = PlanNode::new(
        Operation::Deduplicate {
            input: Box::new(scan("ORDERS")),
            group: vec![column_reference("region")],
            identity: vec![ColumnName::new("order_id")],
        },
        region_schema(),
        query_origin(),
    );
    let stitch = PlanNode::new(
        Operation::Stitch {
            branches: vec![filtered_total(deduplicate), filtered_total(scan("RETURNS"))],
            keys: vec![ColumnName::new("region")],
            columns: vec![StitchColumn {
                name: ColumnName::new("amount"),
                branch: 0,
                empty_value: None,
            }],
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
    assert!(!stitch.is_simple_plan());
    assert!(!first_branch.is_simple_plan());
}

#[test]
fn e4_3_16_e4_4_2_a_lookup_reads_its_result_then_its_value_and_is_not_simple() {
    let lookup = PlanNode::new(
        Operation::Lookup {
            result: Box::new(filtered_total(scan("ORDERS"))),
            value: Box::new(filtered_total(scan("RETURNS"))),
            mapping: vec![LookupKey::Equal {
                result: ColumnName::new("region"),
                value: ColumnName::new("region"),
            }],
            columns: vec![LookupColumn {
                value: ColumnName::new("amount"),
                name: ColumnName::new("context_1_component_1"),
                empty_value: None,
            }],
        },
        region_schema(),
        query_origin(),
    );

    let inputs: Vec<&str> = lookup
        .inputs()
        .into_iter()
        .map(|input| kind_of(input.operation()))
        .collect();
    assert_eq!(inputs.len(), 2);
    assert!(lookup.inputs().iter().all(|input| input.is_simple_plan()));
    assert!(!lookup.is_simple_plan());
}

#[test]
fn e4_4_2_a_plan_without_stitch_and_deduplicate_is_simple() {
    let join = PlanNode::new(
        Operation::Join {
            left: Box::new(scan("ORDERS")),
            right: Box::new(scan("CUSTOMERS")),
            relationship: name("ORDERS_TO_CUSTOMERS"),
            kind: JoinKind::Left,
            on: Vec::new(),
            right_prefix: ColumnPrefix::new("ORDERS_TO_CUSTOMERS."),
        },
        region_schema(),
        query_origin(),
    );

    assert!(join.is_simple_plan());
}

#[test]
fn e4_3_12_population_reads_visible_rows_then_periods() {
    let visible = PlanNode::new(
        Operation::Filter {
            input: Box::new(scan("CUSTOMERS")),
            predicate: true_literal(),
        },
        region_schema(),
        in_model("/semantic_model/0/datasets/1"),
    );
    let periods = PlanNode::new(
        Operation::Scan {
            dataset: name("BALANCES"),
            row_identity: None,
        },
        OutputSchema::new(vec![column("period", LogicalType::Date, false)]),
        in_model("/semantic_model/0/datasets/0"),
    );
    let population = PlanNode::new(
        Operation::Population {
            input: Box::new(visible),
            dataset: name("CUSTOMERS"),
            population: name("ACTIVE_CUSTOMERS"),
            grain: Grain::Month,
            periods: Box::new(periods),
            period: ColumnName::new("period"),
            rule: PopulationRule {
                entity: vec![ColumnName::new("CUSTOMER_ID")],
                membership: Membership::Validity {
                    valid_from: ColumnName::new("VALID_FROM"),
                    valid_to: ColumnName::new("VALID_TO"),
                    partial_period: PartialPeriod::PeriodEnd,
                },
                required_when: Some(column_reference("IS_BILLABLE")),
            },
        },
        OutputSchema::new(vec![
            column("CUSTOMER_ID", count_type(), false),
            column("period", LogicalType::Date, false),
        ]),
        in_model("/semantic_model/0/datasets/1/custom_extensions/0"),
    );

    let kinds: Vec<&str> = population
        .inputs()
        .iter()
        .map(|input| kind_of(input.operation()))
        .collect();
    assert_eq!(kinds, ["Filter", "Scan"]);
    assert_eq!(kind_of(population.operation()), "Population");
    assert!(population.is_simple_plan());
}

#[test]
fn chk_6_6_a_full_join_reads_its_left_then_its_right_input() {
    let in_range = PlanNode::new(
        Operation::PeriodRange {
            input: Box::new(scan("ORDERS")),
            coverage: ColumnName::new("region"),
        },
        region_schema(),
        in_model("/semantic_model/0/datasets/0"),
    );
    let compared = PlanNode::new(
        Operation::FullJoin {
            left: Box::new(in_range),
            right: Box::new(scan("ORDERS_BY_REGION")),
            on: vec![JoinCondition {
                left: ColumnName::new("region"),
                right: ColumnName::new("summary.region"),
            }],
            right_prefix: ColumnPrefix::new("summary."),
        },
        region_schema(),
        in_model("/semantic_model/0/datasets/1"),
    );

    let kinds: Vec<&str> = compared
        .inputs()
        .iter()
        .map(|input| kind_of(input.operation()))
        .collect();
    assert_eq!(kinds, ["PeriodRange", "Scan"]);
    let in_range_inputs: Vec<&str> = compared.inputs()[0]
        .inputs()
        .iter()
        .map(|input| kind_of(input.operation()))
        .collect();
    assert_eq!(in_range_inputs, ["Scan"]);
    assert!(compared.is_simple_plan());
}
