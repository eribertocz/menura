//! The plans of the `Stitch` tests (issue #317): branches over `ORDERS` and the `Stitch` that
//! combines them.

use menura_contracts::types::common::{IntegerBits, LogicalType};
use menura_generator_clickhouse::GeneratedSql;
use menura_model::expression::{IntegerLiteral, Literal};
use menura_representations::logical_plan::{
    Operation, OutputSchema, PlanNode, SchemaColumn, StitchColumn,
};
use menura_representations::names::ColumnName;

use super::{
    aggregate, big_integer, column, component, decimal, from_model, generate, named, query,
    region_count, scan,
};

/// The sum of 128-bit integers.
pub fn huge_integer() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::OneHundredTwentyEight,
        signed: true,
    }
}

/// The amount of `ORDERS` per region.
pub fn region_amount() -> PlanNode {
    aggregate(
        scan("ORDERS", &["AMOUNT", "REGION"]),
        vec![named("REGION", "REGION")],
        vec![component("TOTAL_AMOUNT", "SUM(AMOUNT)", None)],
        vec![
            column("REGION", LogicalType::Text, true),
            column("TOTAL_AMOUNT", decimal(38, 2), true),
        ],
    )
}

/// The number of distinct quantities of `ORDERS` per region.
pub fn region_quantities() -> PlanNode {
    aggregate(
        scan("ORDERS", &["QUANTITY", "REGION"]),
        vec![named("REGION", "REGION")],
        vec![component(
            "DISTINCT_QUANTITIES",
            "COUNT(DISTINCT QUANTITY)",
            None,
        )],
        vec![
            column("REGION", LogicalType::Text, true),
            column("DISTINCT_QUANTITIES", big_integer(), false),
        ],
    )
}

/// The `Stitch` of `branches` by `keys`, producing `schema`: the non-key column at position
/// `i` comes from the branch `owners[i]`, and its empty-input value is `0` when it is one of
/// `counts` and NULL otherwise.
pub fn stitch(
    branches: Vec<PlanNode>,
    keys: &[&str],
    schema: Vec<SchemaColumn>,
    owners: &[usize],
    counts: &[&str],
) -> PlanNode {
    let columns = schema
        .iter()
        .skip(keys.len())
        .zip(owners)
        .map(|(column, branch)| StitchColumn {
            name: column.name.clone(),
            branch: *branch,
            empty_value: counts
                .contains(&column.name.as_str())
                .then(|| IntegerLiteral::new("0").map(Literal::Integer))
                .flatten(),
        })
        .collect();
    PlanNode::new(
        Operation::Stitch {
            branches,
            keys: keys.iter().map(|key| ColumnName::new(key)).collect(),
            columns,
        },
        OutputSchema::new(schema),
        from_model("/semantic_model/0/metrics/0"),
    )
}

/// The columns of the stitch of `region_count` and `region_amount`.
pub fn two_branch_columns() -> Vec<SchemaColumn> {
    vec![
        column("REGION", LogicalType::Text, true),
        column("ORDER_COUNT", big_integer(), false),
        column("TOTAL_QUANTITY", huge_integer(), true),
        column("TOTAL_AMOUNT", decimal(38, 2), true),
    ]
}

/// The SQL of `plan` for the query `REGION` then `metrics`.
pub fn generated(plan: &PlanNode, metrics: &[&str]) -> GeneratedSql {
    generate(plan, &query(&["REGION"], metrics)).expect("SQL")
}

pub fn two_branches() -> PlanNode {
    stitch(
        vec![region_count(), region_amount()],
        &["REGION"],
        two_branch_columns(),
        &[0, 0, 1],
        &["ORDER_COUNT"],
    )
}

pub const TWO_METRICS: [&str; 3] = ["ORDER_COUNT", "TOTAL_QUANTITY", "TOTAL_AMOUNT"];
