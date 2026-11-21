//! Hand-built multi-node plans in the shape of issue #311: `Scan` nodes of join instances,
//! whose columns carry the `right_prefix` of their `Join`, left-deep `Join` nodes with their
//! key pairs, and `Deduplicate` nodes.

use menura_model::internal_model::NormalizedName;
use menura_representations::logical_plan::{
    JoinCondition, JoinKind, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::{ColumnName, ColumnPrefix};

use super::plan::{column, field_type};
use super::{from_model, parse};

/// The home `Scan` of `dataset` that projects `fields`.
pub fn home_scan(dataset: &str, fields: &[&str]) -> PlanNode {
    super::plan::scan(dataset, fields)
}

/// The `Scan` of `dataset` for the join instance whose columns take `prefix`, projecting
/// `fields` as `{prefix}{FIELD}`.
pub fn joined_scan(dataset: &str, prefix: &str, fields: &[&str]) -> PlanNode {
    let columns = fields
        .iter()
        .map(|name| column(&format!("{prefix}{name}"), field_type(name), true))
        .collect();
    PlanNode::new(
        Operation::Scan {
            dataset: NormalizedName::new(dataset),
        },
        OutputSchema::new(columns),
        from_model("/semantic_model/0/datasets/1"),
    )
}

/// The left `Join` of `left` and `right` along `relationship` on the pairs `keys`, the
/// relationship name followed by `.` as the prefix of `right` when `left` is the home `Scan`.
/// Its columns are those of `left`, then those of `right`, made nullable (PLN-1).
pub fn left_join(
    left: PlanNode,
    right: PlanNode,
    relationship: &str,
    keys: &[(&str, &str)],
) -> PlanNode {
    let mut columns = left.schema().columns().to_vec();
    columns.extend(right.schema().columns().iter().map(|column| SchemaColumn {
        nullable: true,
        ..column.clone()
    }));
    let prefix = right_prefix(&right);
    join_of(left, right, (relationship, &prefix), keys, columns)
}

/// The prefix of the columns of the joined `Scan` `right`: its first column up to and with its
/// last `.`.
fn right_prefix(right: &PlanNode) -> String {
    let first = right.schema().columns()[0].name.as_str();
    let end = first.rfind('.').map_or(0, |position| position + 1);
    first[..end].to_owned()
}

/// The left `Join` of `left` and `right` along the relationship and right prefix
/// `relationship`, on `keys`, producing `columns`.
pub fn join_of(
    left: PlanNode,
    right: PlanNode,
    (relationship, prefix): (&str, &str),
    keys: &[(&str, &str)],
    columns: Vec<SchemaColumn>,
) -> PlanNode {
    let on = keys
        .iter()
        .map(|(left, right)| JoinCondition {
            left: ColumnName::new(left),
            right: ColumnName::new(right),
        })
        .collect();
    PlanNode::new(
        Operation::Join {
            left: Box::new(left),
            right: Box::new(right),
            relationship: NormalizedName::new(relationship),
            kind: JoinKind::Left,
            on,
            right_prefix: ColumnPrefix::new(prefix),
        },
        OutputSchema::new(columns),
        from_model("/semantic_model/0/relationships/0"),
    )
}

/// The `Deduplicate` of `input` by the expressions `group` and the columns `identity`, with
/// the schema of its input.
pub fn deduplicate(input: PlanNode, group: &[&str], identity: &[&str]) -> PlanNode {
    let schema = OutputSchema::new(input.schema().columns().to_vec());
    PlanNode::new(
        Operation::Deduplicate {
            input: Box::new(input),
            group: group.iter().map(|text| parse(text)).collect(),
            identity: identity.iter().map(|name| ColumnName::new(name)).collect(),
        },
        schema,
        from_model("/semantic_model/0/relationships/0"),
    )
}
