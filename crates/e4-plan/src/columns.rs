//! Plan columns: their names, references to them, and the fields of the home dataset that
//! expressions read.

use menura_contracts::normalize_unquoted_identifier;
use menura_model::expression::{Expression, Identifier, Reference};
use menura_model::internal_model::NormalizedName;
use menura_representations::names::ColumnName;

use crate::error::InvariantViolation;
use crate::rewrite::rewrite;

/// The column of the Aggregate key of the dimension at `position` among the dimensions,
/// from 0.
pub(crate) fn dimension_column(position: usize) -> ColumnName {
    ColumnName::new(&format!("dimension_{}", position + 1))
}

/// The column of the Aggregate component `component` of the metric `metric`, both from 0.
pub(crate) fn component_column(metric: usize, component: usize) -> ColumnName {
    ColumnName::new(&format!(
        "metric_{}_component_{}",
        metric + 1,
        component + 1
    ))
}

/// The unqualified reference to the input column `name` (issue #251), as a delimited
/// identifier so that it names the column exactly.
///
/// # Errors
///
/// [`InvariantViolation::EmptyName`] when `name` is empty.
pub(crate) fn column_reference(name: &str) -> Result<Expression, InvariantViolation> {
    let identifier = Identifier::delimited(name).ok_or(InvariantViolation::EmptyName)?;
    Ok(Expression::Reference(Reference {
        qualifier: None,
        name: identifier,
    }))
}

/// The field a reference names (E0D-1.1): a regular identifier in upper case, a delimited one
/// exactly.
pub(crate) fn field_name(identifier: &Identifier) -> NormalizedName {
    if identifier.is_delimited() {
        NormalizedName::new(identifier.text())
    } else {
        NormalizedName::new(&normalize_unquoted_identifier(identifier.text()))
    }
}

/// `expression` over the fields of the home dataset, with every field reference turned into
/// the unqualified reference to the `Scan` column of that field. A reference qualified with
/// `source` in a dataset metric, a source column (E0D-3), is not a field.
///
/// # Errors
///
/// [`InvariantViolation::SourceColumn`] for a source column.
pub(crate) fn over_scan(
    expression: &Expression,
    in_dataset_metric: bool,
) -> Result<Expression, InvariantViolation> {
    rewrite(expression, &mut |node| match node {
        Expression::Reference(reference) => {
            let is_source = in_dataset_metric
                && reference
                    .qualifier
                    .as_ref()
                    .is_some_and(|qualifier| field_name(qualifier).as_str() == "SOURCE");
            if is_source {
                return Err(InvariantViolation::SourceColumn);
            }
            column_reference(field_name(&reference.name).as_str()).map(Some)
        }
        _ => Ok(None),
    })
}

/// Adds the columns `expression` reads after [`over_scan`], in written order, to `fields`.
pub(crate) fn read_fields(expression: &Expression, fields: &mut Vec<NormalizedName>) {
    let visited: Result<Expression, std::convert::Infallible> = rewrite(expression, &mut |node| {
        if let Expression::Reference(reference) = node {
            fields.push(NormalizedName::new(reference.name.text()));
        }
        Ok(None)
    });
    drop(visited);
}
