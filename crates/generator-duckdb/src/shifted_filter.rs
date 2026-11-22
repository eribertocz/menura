//! The step of a `ShiftedFilter` node (E4-8.4, E6-4.14): the query filters of a `prior_period`
//! context query and its shifted `date_range`, in one `WHERE` clause.

use menura_model::expression::Expression;
use menura_representations::logical_plan::{PlanNode, ShiftedRange};

use crate::body::SelectBody;
use crate::error::GenerateError;
use crate::expression::Columns;
use crate::steps::{StepWriter, pass_through};

impl<'input> StepWriter<'input> {
    /// The query filters, then the shifted range of a `prior_period` context query, in one
    /// `WHERE` clause: the literals of the filters are bound, then the query's `from` and
    /// `to`, in order, while the amounts of the shift are written (E4-8.4, E6-4.11, E6-4.14).
    pub(crate) fn shifted_filter(
        &mut self,
        node: &'input PlanNode,
        input: &'input PlanNode,
        predicate: Option<&Expression>,
        range: &ShiftedRange,
    ) -> Result<SelectBody, GenerateError> {
        let written = self.write(input)?;
        self.parameters.start_node(node.origin());
        let items = pass_through(node, input.schema())?;
        let mut conditions = Vec::new();
        if let Some(predicate) = predicate {
            let condition = self.expression(node, Columns::Input(input.schema()), predicate)?;
            conditions.push(format!("({condition})"));
        }
        let bounds = self
            .writer(node, Columns::Input(input.schema()))
            .binding_dates_only()
            .write(&range.predicate())?;
        conditions.push(format!("({bounds})"));
        Ok(SelectBody {
            items,
            from: written.name,
            clauses: vec![format!("WHERE {}", conditions.join(" AND "))],
        })
    }
}
