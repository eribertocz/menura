//! The step of a `FullJoin` node (CHK-6.6): the columns of both inputs, matched on the key
//! pairs with `IS NOT DISTINCT FROM`, so that NULL values match.

use menura_representations::logical_plan::{JoinCondition, OutputSchema, PlanNode};
use menura_representations::names::{ColumnName, ColumnPrefix};

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::join_keys::{KeySide, null_safe_key_equality};
use crate::steps::{StepWriter, error};

impl<'input> StepWriter<'input> {
    /// The steps of `left`, then of `right` with its columns under `right_prefix` (E6-4.12),
    /// then the step that selects every column of `node` from them, full outer joined on the
    /// key pairs.
    pub(crate) fn full_join(
        &mut self,
        node: &'input PlanNode,
        (left, right): (&'input PlanNode, &'input PlanNode),
        on: &'input [JoinCondition],
        right_prefix: &'input ColumnPrefix,
    ) -> Result<SelectBody, GenerateError> {
        if on.is_empty() {
            return Err(error(InvariantViolation::JoinWithoutKeys, node));
        }
        let left_step = self.write(left)?;
        let right_step = self.write(right)?;
        self.parameters.start_node(node.origin());
        let steps = (&left_step.name, &right_step.name);
        let items = node
            .schema()
            .columns()
            .iter()
            .map(|column| joined_item(node, (left, right), steps, right_prefix, &column.name))
            .collect::<Result<Vec<_>, _>>()?;
        let conditions = on
            .iter()
            .map(|pair| {
                let unprefixed = unprefixed(node, right_prefix, &pair.right)?;
                let left_side = side(node, left.schema(), steps.0, &pair.left)?;
                let right_side = side(node, right.schema(), steps.1, &unprefixed)?;
                null_safe_key_equality(left_side.borrow(), right_side.borrow())
                    .ok_or_else(|| error(InvariantViolation::IncompatibleJoinKeys, node))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let condition = conditions
            .iter()
            .map(|condition| format!("({condition})"))
            .collect::<Vec<_>>()
            .join(" AND ");
        Ok(SelectBody {
            items,
            from: left_step.name,
            clauses: vec![format!(
                "FULL OUTER JOIN {} ON {condition}",
                right_step.name
            )],
        })
    }
}

/// The select item of the output column `name`: the column of that name of `left`, or the
/// column of `right` that `name` holds under `right_prefix`.
fn joined_item(
    node: &PlanNode,
    (left, right): (&PlanNode, &PlanNode),
    (left_step, right_step): (&String, &String),
    right_prefix: &ColumnPrefix,
    name: &ColumnName,
) -> Result<String, GenerateError> {
    let (step, read) = if has(left.schema(), name) {
        (left_step, name.clone())
    } else {
        let read = unprefixed(node, right_prefix, name)?;
        if !has(right.schema(), &read) {
            return Err(error(InvariantViolation::UnknownColumn, node));
        }
        (right_step, read)
    };
    Ok(format!(
        "{step}.{} AS {}",
        quote(read.as_str()),
        quote(name.as_str())
    ))
}

/// The column of the right input that the output column `name` holds under `right_prefix`.
fn unprefixed(
    node: &PlanNode,
    right_prefix: &ColumnPrefix,
    name: &ColumnName,
) -> Result<ColumnName, GenerateError> {
    name.as_str()
        .strip_prefix(right_prefix.as_str())
        .map(ColumnName::new)
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))
}

/// A key column, qualified by the name of its step, with its type.
struct OwnedSide<'input> {
    sql: String,
    logical_type: &'input menura_contracts::types::common::LogicalType,
}

impl OwnedSide<'_> {
    fn borrow(&self) -> KeySide<'_> {
        KeySide {
            sql: &self.sql,
            logical_type: self.logical_type,
        }
    }
}

fn side<'input>(
    node: &PlanNode,
    schema: &'input OutputSchema,
    step: &str,
    name: &ColumnName,
) -> Result<OwnedSide<'input>, GenerateError> {
    let column = schema
        .columns()
        .iter()
        .find(|column| &column.name == name)
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))?;
    Ok(OwnedSide {
        sql: format!("{step}.{}", quote(name.as_str())),
        logical_type: &column.logical_type,
    })
}

fn has(schema: &OutputSchema, name: &ColumnName) -> bool {
    schema.columns().iter().any(|column| &column.name == name)
}
