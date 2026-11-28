//! Builders of the branch plans of the summary tests (E4-4).

use menura_contracts::types::common::LogicalType;
use menura_model::expression::{BinaryOperator, DateUnit, Expression, SimpleAggregate};
use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{
    AggregateComponent, JoinCondition, JoinKind, NamedExpression, Operation, OutputSchema,
    PlanNode, SchemaColumn,
};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;

use super::expressions::{binary, call, column, equals, text, truncated};
use super::{ORDERS, at, big_integer, location, type_of};

fn schema_column(name: &str, logical_type: LogicalType) -> SchemaColumn {
    SchemaColumn {
        name: ColumnName::new(name),
        logical_type,
        nullable: true,
    }
}

/// The `Scan` of `dataset` that projects `fields`, in that order.
pub(in super::super) fn scan(dataset: &str, fields: &[&str]) -> PlanNode {
    let columns = fields
        .iter()
        .map(|field| schema_column(field, type_of(dataset, field)))
        .collect();
    PlanNode::new(
        Operation::Scan {
            dataset: NormalizedName::new(dataset),
            row_identity: None,
        },
        OutputSchema::new(columns),
        Origin::Model(location()),
    )
}

/// The `Scan` of the join instance of `CUSTOMERS` under the prefix `j1.`.
pub(in super::super) fn customers() -> PlanNode {
    let prefix = ColumnPrefix::new("j1.");
    let columns = ["CUSTOMER_ID", "COUNTRY"]
        .into_iter()
        .map(|field| schema_column(prefix.column(field).as_str(), type_of("CUSTOMERS", field)))
        .collect();
    PlanNode::new(
        Operation::Scan {
            dataset: NormalizedName::new("CUSTOMERS"),
            row_identity: None,
        },
        OutputSchema::new(columns),
        Origin::Model(location()),
    )
}

/// `Filter(input, predicate)` from `origin`.
pub(in super::super) fn filter_from(
    input: PlanNode,
    predicate: Expression,
    origin: Origin,
) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Filter {
            input: Box::new(input),
            predicate,
        },
        schema,
        origin,
    )
}

/// The row-policy `Filter` of the home dataset.
pub(in super::super) fn policy(input: PlanNode, predicate: Expression) -> PlanNode {
    filter_from(input, predicate, Origin::Model(location()))
}

/// The query-filter `Filter`.
pub(in super::super) fn query_filter(input: PlanNode, predicate: Expression) -> PlanNode {
    filter_from(input, predicate, Origin::Query(at()))
}

/// The propagation `SemiJoin` of `input` to `CUSTOMERS` from its column `from`.
pub(in super::super) fn semi_join(input: PlanNode, from: &str) -> PlanNode {
    let schema = input.schema().clone();
    let related = scan("CUSTOMERS", &["CUSTOMER_ID", "COUNTRY"]);
    PlanNode::new(
        Operation::SemiJoin {
            input: Box::new(input),
            relationship: NormalizedName::new("ORDERS_TO_CUSTOMERS"),
            on: vec![JoinCondition {
                left: ColumnName::new(from),
                right: ColumnName::new("CUSTOMER_ID"),
            }],
            temporal: None,
            related_all: Box::new(related.clone()),
            related_visible: Box::new(policy(related, equals("COUNTRY", "FR"))),
        },
        schema,
        Origin::Model(location()),
    )
}

/// The `Join` of `left` to the instance of `CUSTOMERS` from its column `from`.
pub(in super::super) fn join(left: PlanNode, from: &str) -> PlanNode {
    let right = customers();
    let mut columns = left.schema().columns().to_vec();
    columns.extend(right.schema().columns().iter().cloned());
    PlanNode::new(
        Operation::Join {
            left: Box::new(left),
            right: Box::new(right),
            relationship: NormalizedName::new("ORDERS_TO_CUSTOMERS"),
            kind: JoinKind::Left,
            on: vec![JoinCondition {
                left: ColumnName::new(from),
                right: ColumnName::new("j1.CUSTOMER_ID"),
            }],
            temporal: None,
            right_prefix: ColumnPrefix::new("j1."),
        },
        OutputSchema::new(columns),
        Origin::Model(location()),
    )
}

/// The `Deduplicate` of `input` per home row.
pub(in super::super) fn deduplicate(input: PlanNode) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Deduplicate {
            input: Box::new(input),
            group: Vec::new(),
            identity: vec![ColumnName::new("menura.row")],
        },
        schema,
        Origin::Model(location()),
    )
}

/// The component `name` computing `function` over the column `argument`.
pub(in super::super) fn component(
    name: &str,
    function: SimpleAggregate,
    argument: &str,
) -> AggregateComponent {
    AggregateComponent {
        name: ColumnName::new(name),
        call: call(function, column(argument)),
        filter: None,
        recomposition: None,
    }
}

/// The `Aggregate` of `input` by `keys`, (name, expression), computing `components`.
pub(in super::super) fn aggregate(
    input: PlanNode,
    keys: &[(&str, Expression)],
    components: Vec<AggregateComponent>,
) -> PlanNode {
    let mut columns: Vec<SchemaColumn> = keys
        .iter()
        .map(|(name, _)| schema_column(name, LogicalType::Text))
        .collect();
    columns.extend(
        components
            .iter()
            .map(|component| schema_column(component.name.as_str(), big_integer())),
    );
    PlanNode::new(
        Operation::Aggregate {
            input: Box::new(input),
            keys: keys
                .iter()
                .map(|(name, expression)| NamedExpression {
                    name: ColumnName::new(name),
                    expression: expression.clone(),
                    derived: None,
                })
                .collect(),
            components,
        },
        OutputSchema::new(columns),
        Origin::Model(location()),
    )
}

/// The `Aggregate` of the sum of `AMOUNT` per region and month over `input`.
pub(in super::super) fn by_region_and_month(input: PlanNode) -> PlanNode {
    aggregate(
        input,
        &[
            ("dimension_1", column("REGION")),
            ("dimension_2", truncated(DateUnit::Month, "ORDER_DATE")),
        ],
        vec![component("total", SimpleAggregate::Sum, "AMOUNT")],
    )
}

/// The home `Scan` of `ORDERS` read by `by_region_and_month`, under a row policy on `REGION`.
pub(in super::super) fn visible_orders() -> PlanNode {
    policy(
        scan(ORDERS, &["REGION", "ORDER_DATE", "CUSTOMER_ID", "AMOUNT"]),
        binary(BinaryOperator::NotEqual, column("REGION"), text("XX")),
    )
}
