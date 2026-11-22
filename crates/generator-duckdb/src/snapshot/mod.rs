//! The step of a `Snapshot` node (E4-3.13, E4-7, E6-4.14): the rows of its input reduced to
//! one value per (result group, entity) under a semi-additive rule.

mod field_over;
mod nulls;

use menura_representations::logical_plan::{NamedExpression, Operation, PlanNode, SnapshotOver};

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::identifiers::quote;
use crate::steps::{StepWriter, error};

impl<'input> StepWriter<'input> {
    /// The steps of the node's input, then those of its rule; the body returned is the last.
    pub(crate) fn snapshot(&mut self, node: &'input PlanNode) -> Result<SelectBody, GenerateError> {
        let Operation::Snapshot { input, rule } = node.operation() else {
            return Err(error(InvariantViolation::NodeOutsideSlice, node));
        };
        match &rule.over {
            SnapshotOver::Field(over) => self.field_snapshot(node, input, rule, over),
            SnapshotOver::Time(_) => Err(error(InvariantViolation::NodeOutsideSlice, node)),
        }
    }

    /// `expression AS name` for each of `named`, over the columns of `input`.
    fn named_items(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        named: &[NamedExpression],
    ) -> Result<Vec<String>, GenerateError> {
        named
            .iter()
            .map(|item| {
                let value =
                    self.expression(node, Columns::Input(input.schema()), &item.expression)?;
                Ok(format!("{value} AS {}", quote(item.name.as_str())))
            })
            .collect()
    }
}

/// `GROUP BY 1, 2, …, count`, or no clause when `count` is 0.
fn group_by(count: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }
    let ordinals: Vec<String> = (1..=count).map(|ordinal| ordinal.to_string()).collect();
    vec![format!("GROUP BY {}", ordinals.join(", "))]
}
