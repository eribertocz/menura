//! The columns a check computes, typed as E6 types their expressions (CHK-4.2), so that a check
//! column has exactly the type a query column of the same expression has.

use menura_generator_duckdb::check_column_type;
use menura_model::expression::Expression;
use menura_representations::logical_plan::{OutputSchema, SchemaColumn};
use menura_representations::names::ColumnName;

use super::key_columns::STAGE;
use crate::compile::InternalError;

/// The column `name` computed by `expression` over the columns of `input`: its type and
/// nullability as E0.f types the expression (E0F-5, E0F-7.1).
pub(super) fn typed_column(
    name: &ColumnName,
    expression: &Expression,
    input: &OutputSchema,
) -> Result<SchemaColumn, InternalError> {
    let (logical_type, nullable) = check_column_type(expression, input).ok_or_else(|| {
        InternalError::new(
            STAGE,
            format!("the check column {} has no type", name.as_str()),
        )
    })?;
    Ok(SchemaColumn {
        name: name.clone(),
        logical_type,
        nullable,
    })
}
