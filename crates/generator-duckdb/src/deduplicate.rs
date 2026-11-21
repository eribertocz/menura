//! The step of a `Deduplicate` node (E4-3.5, E6-4.14): one row per (`group`, `identity`) pair.

use menura_model::expression::Expression;
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::ColumnName;

use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::identifiers::quote;
use crate::steps::{StepWriter, error, pass_through};

impl<'input> StepWriter<'input> {
    /// The columns of `input`, keeping the first row of each partition by the `group`
    /// expressions, already truncated to their grains, then the `identity` columns. `PARTITION BY` compares NULLs as equal, as `group` requires
    /// (E4-4.0f); the rows of one partition share their home row, so which one is kept does
    /// not change the result.
    pub(crate) fn deduplicate(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        group: &[Expression],
        identity: &[ColumnName],
    ) -> Result<SelectBody, GenerateError> {
        if identity.is_empty() {
            return Err(error(InvariantViolation::EmptyIdentity, node));
        }
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let mut partition = Vec::new();
        for expression in group {
            partition.push(self.expression(node, Columns::Input(input.schema()), expression)?);
        }
        for name in identity {
            partition.push(input_column(node, input, name)?);
        }
        Ok(SelectBody {
            items: pass_through(node, input.schema())?,
            from: written.name,
            clauses: vec![format!(
                "QUALIFY ROW_NUMBER() OVER (PARTITION BY {}) = 1",
                partition.join(", ")
            )],
        })
    }
}

/// The quoted column `name` of `input`.
fn input_column(
    node: &PlanNode,
    input: &PlanNode,
    name: &ColumnName,
) -> Result<String, GenerateError> {
    input
        .schema()
        .columns()
        .iter()
        .any(|column| &column.name == name)
        .then(|| quote(name.as_str()))
        .ok_or_else(|| error(InvariantViolation::UnknownColumn, node))
}
