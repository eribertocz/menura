//! Hand-built plan nodes that follow the plan conventions of issue #251: a `Scan` projects
//! fields by name, and every other node reads the columns of its input.

use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::{AggregateCall, Expression};
use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{
    AggregateComponent, NamedExpression, Operation, OutputSchema, PlanNode, SchemaColumn, SortItem,
};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::{big_integer, from_model, from_query, order_fields, parse};

/// A plan column.
pub fn column(name: &str, logical_type: LogicalType, nullable: bool) -> SchemaColumn {
    SchemaColumn {
        name: ColumnName::new(name),
        logical_type,
        nullable,
    }
}

/// The type of the field `name` of `ORDERS`.
pub fn field_type(name: &str) -> LogicalType {
    order_fields()
        .into_iter()
        .find(|(field, _, _)| *field == name)
        .map(|(_, _, logical_type)| logical_type)
        .expect("a field of ORDERS")
}

/// The `Scan` of `dataset` that projects `fields`.
pub fn scan(dataset: &str, fields: &[&str]) -> PlanNode {
    let columns = fields
        .iter()
        .map(|name| column(name, field_type(name), true))
        .collect();
    scan_of(dataset, columns)
}

/// The `Scan` of `dataset` whose schema is `columns`.
pub fn scan_of(dataset: &str, columns: Vec<SchemaColumn>) -> PlanNode {
    PlanNode::new(
        Operation::Scan {
            dataset: NormalizedName::new(dataset),
        },
        OutputSchema::new(columns),
        from_model("/semantic_model/0/datasets/0"),
    )
}

/// The `Filter` of `input` by `predicate`, from `origin`.
pub fn filter(input: PlanNode, predicate: &str, origin: Origin) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Filter {
            input: Box::new(input),
            predicate: parse(predicate),
        },
        schema,
        origin,
    )
}

/// The named expression `name` = `expression`.
pub fn named(name: &str, expression: &str) -> NamedExpression {
    NamedExpression {
        name: ColumnName::new(name),
        expression: parse(expression),
    }
}

/// The component `name` computed by the aggregate `call`, with an optional filter.
pub fn component(name: &str, call: &str, filter: Option<&str>) -> AggregateComponent {
    let Expression::Aggregate { call, .. } = parse(call) else {
        panic!("{call} is not an aggregate");
    };
    component_of(name, call, filter)
}

/// The component `name` computed by `call`, with an optional filter.
pub fn component_of(name: &str, call: AggregateCall, filter: Option<&str>) -> AggregateComponent {
    AggregateComponent {
        name: ColumnName::new(name),
        call,
        filter: filter.map(parse),
    }
}

/// The `Aggregate` of `input`, producing `schema`, from the query.
pub fn aggregate(
    input: PlanNode,
    keys: Vec<NamedExpression>,
    components: Vec<AggregateComponent>,
    schema: Vec<SchemaColumn>,
) -> PlanNode {
    PlanNode::new(
        Operation::Aggregate {
            input: Box::new(input),
            keys,
            components,
        },
        OutputSchema::new(schema),
        from_query("/query"),
    )
}

/// The `Project` of `input` that computes `expressions`, producing `schema`, from `origin`.
pub fn project(
    input: PlanNode,
    expressions: Vec<NamedExpression>,
    schema: Vec<SchemaColumn>,
    origin: Origin,
) -> PlanNode {
    PlanNode::new(
        Operation::Project {
            input: Box::new(input),
            expressions,
        },
        OutputSchema::new(schema),
        origin,
    )
}

/// The `Sort` of `input` by `items`: column, descending, NULLs first.
pub fn sort(input: PlanNode, items: &[(&str, bool, bool)]) -> PlanNode {
    let schema = input.schema().clone();
    let items = items
        .iter()
        .map(|(column, descending, nulls_first)| SortItem {
            column: ColumnName::new(column),
            direction: if *descending {
                Direction::Desc
            } else {
                Direction::Asc
            },
            nulls: if *nulls_first {
                NullsOrder::First
            } else {
                NullsOrder::Last
            },
        })
        .collect();
    PlanNode::new(
        Operation::Sort {
            input: Box::new(input),
            items,
        },
        schema,
        from_query("/query/order"),
    )
}

/// The `Limit` of `input` to `rows`.
pub fn limit(input: PlanNode, rows: u64) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Limit {
            input: Box::new(input),
            rows,
        },
        schema,
        from_query("/query/limit"),
    )
}

/// The `Aggregate` of `ORDERS` that counts orders and sums quantities per region.
pub fn region_count() -> PlanNode {
    aggregate(
        scan("ORDERS", &["QUANTITY", "REGION"]),
        vec![named("REGION", "REGION")],
        vec![
            component("ORDER_COUNT", "COUNT(*)", None),
            component("TOTAL_QUANTITY", "SUM(QUANTITY)", None),
        ],
        vec![
            column("REGION", LogicalType::Text, true),
            column("ORDER_COUNT", big_integer(), false),
            column(
                "TOTAL_QUANTITY",
                LogicalType::Integer {
                    bits: IntegerBits::OneHundredTwentyEight,
                    signed: true,
                },
                true,
            ),
        ],
    )
}
