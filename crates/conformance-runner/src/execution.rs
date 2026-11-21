//! Executes the SQL of a compiled plan on the prepared database of a case, with its parameters
//! bound by their logical types (E6-5.4).

use duckdb::Connection;
use duckdb::types::Value;
use menura_core::CompiledPlanDocument;

use crate::parameter_values::parameter_values;

/// Why a plan does not execute.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecutionFailure {
    /// A parameter cannot be bound: a defect of the plan or a type the runner cannot bind.
    Parameter(String),
    /// The engine rejects the statement or fails while running it, with its message.
    Engine(String),
}

/// The rows the SQL of `plan` returns on `connection`, each with one value per column.
///
/// # Errors
///
/// Returns an [`ExecutionFailure`] when a parameter cannot be bound or the engine fails.
pub(crate) fn execute_plan(
    connection: &Connection,
    plan: &CompiledPlanDocument,
) -> Result<Vec<Vec<Value>>, ExecutionFailure> {
    let values = parameter_values(&plan.parameters).map_err(ExecutionFailure::Parameter)?;
    let engine = |error: duckdb::Error| ExecutionFailure::Engine(error.to_string());
    let mut statement = connection.prepare(&plan.sql).map_err(engine)?;
    let mut rows = statement
        .query(duckdb::params_from_iter(values.iter()))
        .map_err(engine)?;
    let mut result = Vec::new();
    while let Some(row) = rows.next().map_err(engine)? {
        let count = row.as_ref().column_count();
        result.push(
            (0..count)
                .map(|index| row.get::<_, Value>(index))
                .collect::<Result<Vec<_>, _>>()
                .map_err(engine)?,
        );
    }
    Ok(result)
}
