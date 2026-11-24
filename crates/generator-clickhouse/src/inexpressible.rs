//! The check of E6-7.12: a plan that reads or returns a value of a type `ClickHouse` cannot
//! express stops with `E6-01` before any SQL is written.

use menura_representations::logical_plan::PlanNode;

use crate::error::GenerateError;
use crate::types::is_inexpressible;

/// `E6-01` at the first node of `plan`, in the order its steps are written (E6-4.12,
/// depth-first post-order, inputs in operand order), that reads or returns a column of a type
/// `ClickHouse` cannot express: a `Scan` reads the fields of its schema, every node returns
/// the columns of its schema. The error is located at that node's origin, the field or metric
/// it comes from.
///
/// # Errors
///
/// [`GenerateError::InexpressibleType`] when there is such a node.
pub(crate) fn check_expressible(plan: &PlanNode) -> Result<(), GenerateError> {
    for input in plan.inputs() {
        check_expressible(input)?;
    }
    let inexpressible = plan
        .schema()
        .columns()
        .iter()
        .any(|column| is_inexpressible(&column.logical_type));
    if inexpressible {
        return Err(GenerateError::InexpressibleType {
            origin: plan.origin().clone(),
        });
    }
    Ok(())
}
