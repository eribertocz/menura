//! The result schema (PLN-1): the columns of the final node, with their roles from the query.

use menura_contracts::types::compiled_plan::{ColumnRole, ResultColumn};
use menura_representations::bound_query::BoundQuery;
use menura_representations::columns::OutputColumn;
use menura_representations::logical_plan::PlanNode;

use crate::error::{GenerateError, InvariantViolation};
use crate::types::column_type;

/// The output columns of `plan`, the final node, whose columns carry the output names in
/// output order (E1-5.1); each role is that of the query column at the same position.
pub(crate) fn result_schema(
    plan: &PlanNode,
    query: &BoundQuery,
) -> Result<Vec<ResultColumn>, GenerateError> {
    let error = |violation| GenerateError::new(violation, plan.origin());
    let columns = plan.schema().columns();
    if columns.len() != query.columns().iter().count() {
        return Err(error(InvariantViolation::ResultColumnMismatch));
    }
    columns
        .iter()
        .zip(query.columns().iter())
        .map(|(column, output)| {
            let role = match output {
                OutputColumn::Dimension(_) => ColumnRole::Dimension,
                OutputColumn::Metric(_) => ColumnRole::Metric,
            };
            Ok(ResultColumn {
                name: column.name.as_str().to_owned(),
                role,
                logical_type: column.logical_type.clone(),
                physical_type: column_type(&column.logical_type, column.nullable)
                    .ok_or_else(|| error(InvariantViolation::UnsupportedType))?,
                nullable: column.nullable,
            })
        })
        .collect()
}
