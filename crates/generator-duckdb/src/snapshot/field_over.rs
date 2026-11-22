//! A `Snapshot` across a field that is not a time field (ADD-5.1.5, E4-7.1): the components per
//! (result group, entity, `over` value), then `min`, `max` or `avg` per (result group, entity).

use menura_model::expression::Expression;
use menura_model::internal_model::{NullHandling, SnapshotAggregation};
use menura_representations::logical_plan::{PlanNode, SnapshotRule};

use super::group_by;
use super::nulls::handle_null;
use crate::body::SelectBody;
use crate::error::{GenerateError, InvariantViolation};
use crate::expression::Columns;
use crate::identifiers::quote;
use crate::steps::{StepWriter, column_type, error};

/// The synthesized column of the `over` value (E6-4.12).
const OVER: &str = "\"menura.over\"";

impl<'input> StepWriter<'input> {
    /// A `values` step with one row per (keys, entity, `over` value) and its components after
    /// `nulls`, then the body that combines each entity's values with `aggregation`.
    pub(super) fn field_snapshot(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        rule: &SnapshotRule,
        over: &Expression,
    ) -> Result<SelectBody, GenerateError> {
        let function = field_function(node, rule)?;
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let columns = Columns::Input(input.schema());
        let mut items = self.named_items(node, input, &rule.keys)?;
        items.extend(self.named_items(node, input, &rule.entity)?);
        let over = self.expression(node, columns, over)?;
        items.push(format!("{over} AS {OVER}"));
        let grouped = items.len();
        for component in &rule.components {
            let call = self
                .writer(node, columns)
                .aggregate(&component.call, component.filter.as_ref())?;
            items.push(format!(
                "{} AS {}",
                handle_null(&call, rule.nulls, &rule.metric),
                quote(component.name.as_str())
            ));
        }
        let values = SelectBody {
            items,
            from: written.name,
            clauses: group_by(grouped),
        };
        let values = self.push_step("values", values);
        let mut items: Vec<String> = rule
            .keys
            .iter()
            .chain(&rule.entity)
            .map(|item| quote(item.name.as_str()))
            .collect();
        for component in &rule.components {
            let name = quote(component.name.as_str());
            let result_type = column_type(node, component.name.as_str())?;
            items.push(format!(
                "CAST({function}({name}) AS {result_type}) AS {name}"
            ));
        }
        Ok(SelectBody {
            items,
            from: values,
            clauses: group_by(rule.keys.len() + rule.entity.len()),
        })
    }
}

/// The SQL function of `aggregation` across a field: `first`, `last` and `nulls: as_missing`
/// need a time `over` (ADD-5.1.5).
fn field_function(node: &PlanNode, rule: &SnapshotRule) -> Result<&'static str, GenerateError> {
    if rule.nulls == NullHandling::AsMissing {
        return Err(error(InvariantViolation::SnapshotOutsideRule, node));
    }
    match rule.aggregation {
        SnapshotAggregation::Minimum => Ok("MIN"),
        SnapshotAggregation::Maximum => Ok("MAX"),
        SnapshotAggregation::Average => Ok("AVG"),
        SnapshotAggregation::First(_) | SnapshotAggregation::Last(_) => {
            Err(error(InvariantViolation::SnapshotOutsideRule, node))
        }
    }
}
