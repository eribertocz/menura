//! The type of an expression over the columns of an input node.

use menura_expressions::typing::{Aggregates, Stop, Typed, TypingScope, type_expression};
use menura_model::expression::{Expression, Reference};
use menura_representations::logical_plan::{OutputSchema, SchemaColumn};
use menura_representations::names::ColumnName;

use crate::error::InvariantViolation;

/// The column `name` that computes `expression`, typed over the columns of `input` (E4-3).
///
/// # Errors
///
/// [`InvariantViolation::UntypedExpression`] when the expression has no logical type there.
pub(crate) fn typed_column(
    name: ColumnName,
    expression: &Expression,
    input: &OutputSchema,
) -> Result<SchemaColumn, InvariantViolation> {
    let resolve = |reference: &Reference| {
        input
            .columns()
            .iter()
            .find(|column| {
                reference.qualifier.is_none() && column.name.as_str() == reference.name.text()
            })
            .map(|column| Typed::value(column.logical_type.clone(), column.nullable))
            .ok_or(Stop::ReportedElsewhere)
    };
    let scope = TypingScope::new(&resolve, Aggregates::Allowed);
    let typed =
        type_expression(expression, &scope).map_err(|_| InvariantViolation::UntypedExpression)?;
    let logical_type = typed
        .logical()
        .cloned()
        .ok_or(InvariantViolation::UntypedExpression)?;
    Ok(SchemaColumn {
        name,
        logical_type,
        nullable: typed.nullable,
    })
}
