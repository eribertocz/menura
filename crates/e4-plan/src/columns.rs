//! Plan columns: their names, references to them, and the fields and source columns of the
//! home dataset that expressions read.

use menura_contracts::normalize_unquoted_identifier;
use menura_model::expression::{Expression, Identifier, Reference};
use menura_model::internal_model::{Dataset, NormalizedName};
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

/// The column of the value of the metric `metric`, from 0, computed below the `Project` of the
/// inline metrics of the SQL form, which reads it.
pub(crate) fn metric_value_column(metric: usize) -> ColumnName {
    ColumnName::new(&format!("metric_{}", metric + 1))
}

/// The column of the input `input` of the metric `metric`, both from 0: the value of an input
/// of a derived metric, computed below the final `Project` (DER-3.1).
pub(crate) fn input_column(metric: usize, input: usize) -> ColumnName {
    ColumnName::new(&format!("metric_{}_input_{}", metric + 1, input + 1))
}

/// The shortest `menura.source` followed by one or more `.` that no field name of `home`
/// starts with: the prefix of every `Scan` column of `home` that names a catalog column of its
/// `source` rather than a field, so that the two never collide (issue #264).
pub(crate) fn source_prefix(home: &Dataset) -> String {
    let mut prefix = String::from("menura.source.");
    while home
        .fields
        .iter()
        .any(|field| field.name.as_str().starts_with(&prefix))
    {
        prefix.push('.');
    }
    prefix
}

/// The `Scan` column of the source column a dataset metric reads as `source.<name>`, where
/// `name` is the reference's normalized name (E0D-1.1), under the [`source_prefix`] `prefix`.
pub(crate) fn source_column(prefix: &str, name: &NormalizedName) -> ColumnName {
    ColumnName::new(&format!("{prefix}{}", name.as_str()))
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

/// `expression` over the `Scan` of the home dataset, with every field reference turned into
/// the unqualified reference to the `Scan` column of that field. A reference qualified with
/// `source` in a dataset metric, whose [`source_prefix`] is `dataset_metric_prefix`, reads a
/// source column (E0D-3), not a field: it becomes the reference to the [`source_column`] of
/// that column. Outside a dataset metric `dataset_metric_prefix` is `None`.
///
/// # Errors
///
/// [`InvariantViolation::EmptyName`] for a reference with an empty name.
pub(crate) fn over_scan(
    expression: &Expression,
    dataset_metric_prefix: Option<&str>,
) -> Result<Expression, InvariantViolation> {
    rewrite(expression, &mut |node| match node {
        Expression::Reference(reference) => {
            let name = field_name(&reference.name);
            let is_source = reference
                .qualifier
                .as_ref()
                .is_some_and(|qualifier| field_name(qualifier).as_str() == "SOURCE");
            match dataset_metric_prefix {
                Some(prefix) if is_source => {
                    column_reference(source_column(prefix, &name).as_str()).map(Some)
                }
                _ => column_reference(name.as_str()).map(Some),
            }
        }
        _ => Ok(None),
    })
}

/// Adds the columns `expression` reads after [`over_scan`], in written order, to `fields`:
/// field names and [`source_column`] names.
pub(crate) fn read_fields(expression: &Expression, fields: &mut Vec<NormalizedName>) {
    let visited: Result<Expression, std::convert::Infallible> = rewrite(expression, &mut |node| {
        if let Expression::Reference(reference) = node {
            fields.push(NormalizedName::new(reference.name.text()));
        }
        Ok(None)
    });
    drop(visited);
}
