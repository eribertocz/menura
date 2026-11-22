//! The type of a column an integrity-check plan computes (CHK-4.2): the type E0.f gives its
//! expression over the columns of its input, the same typer the generator reads (E0F-5,
//! E0F-7.1), so that a check column and a query column of the same expression have one type.

use menura_contracts::types::common::LogicalType;
use menura_expressions::typing::{Aggregates, Outcome, Stop, Typed, TypingScope, type_expression};
use menura_model::expression::{Expression, Reference};
use menura_representations::logical_plan::OutputSchema;

/// The logical type of `expression` over the columns of `input`, unqualified references
/// naming its columns, and whether it can be NULL; `None` when it has no type, such as the
/// literal `NULL` or a reference to an unknown column.
///
/// # Example
///
/// ```
/// use menura_contracts::types::common::LogicalType;
/// use menura_generator_duckdb::check_column_type;
/// use menura_model::expression::{Expression, Identifier, Reference};
/// use menura_representations::logical_plan::{OutputSchema, SchemaColumn};
/// use menura_representations::names::ColumnName;
///
/// let input = OutputSchema::new(vec![SchemaColumn {
///     name: ColumnName::new("DAY"),
///     logical_type: LogicalType::Date,
///     nullable: false,
/// }]);
/// let day = Expression::Reference(Reference {
///     qualifier: None,
///     name: Identifier::delimited("DAY").unwrap(),
/// });
/// assert_eq!(check_column_type(&day, &input), Some((LogicalType::Date, false)));
/// ```
#[must_use]
pub fn check_column_type(
    expression: &Expression,
    input: &OutputSchema,
) -> Option<(LogicalType, bool)> {
    let resolve = |reference: &Reference| -> Outcome<Typed> {
        if reference.qualifier.is_some() {
            return Err(Stop::ReportedElsewhere);
        }
        input
            .columns()
            .iter()
            .find(|column| column.name.as_str() == reference.name.text())
            .map(|column| Typed::value(column.logical_type.clone(), column.nullable))
            .ok_or(Stop::ReportedElsewhere)
    };
    let scope = TypingScope::new(&resolve, Aggregates::Allowed);
    let typed = type_expression(expression, &scope).ok()?;
    Some((typed.logical()?.clone(), typed.nullable))
}
