//! The CAT-8 rules of the catalog schema document: CAT-8.1 to CAT-8.3.

use super::{RuleViolation, first_repeated};
use crate::identifier::normalize_unquoted;
use crate::types::catalog::{CatalogSchemaDocument, RelationSchema};
use crate::types::common::LogicalType;

/// Checks the rules in CAT-8 order; within a rule, in document order.
pub(super) fn check(schema: &CatalogSchemaDocument) -> Result<(), RuleViolation> {
    unique_sources(schema)?;
    for (index, relation) in schema.relations.iter().enumerate() {
        unique_column_names(index, relation)?;
    }
    for (index, relation) in schema.relations.iter().enumerate() {
        decimal_scales(index, relation)?;
    }
    Ok(())
}

/// CAT-8.1: `source` is unique within the document.
fn unique_sources(schema: &CatalogSchemaDocument) -> Result<(), RuleViolation> {
    let sources = schema
        .relations
        .iter()
        .map(|relation| relation.source.as_str());
    match first_repeated(sources) {
        Some(index) => Err(RuleViolation::new(
            format!("/relations/{index}/source"),
            format!(
                "relation `{}` is described more than once",
                schema.relations[index].source
            ),
        )),
        None => Ok(()),
    }
}

/// CAT-8.2: column names are unique within a relation after normalization as an unquoted
/// identifier (E0D-1.1), so `amount` and `AMOUNT` are the same name.
fn unique_column_names(index: usize, relation: &RelationSchema) -> Result<(), RuleViolation> {
    let names = relation
        .columns
        .iter()
        .map(|column| normalize_unquoted(&column.name));
    match first_repeated(names) {
        Some(column) => Err(RuleViolation::new(
            format!("/relations/{index}/columns/{column}/name"),
            format!(
                "column `{}` repeats a column name of relation `{}` after normalization",
                relation.columns[column].name, relation.source
            ),
        )),
        None => Ok(()),
    }
}

/// CAT-8.3: for `decimal`, `scale` ≤ `precision`, including list elements.
fn decimal_scales(index: usize, relation: &RelationSchema) -> Result<(), RuleViolation> {
    for (column_index, column) in relation.columns.iter().enumerate() {
        let pointer = format!("/relations/{index}/columns/{column_index}/logical_type");
        if let Some(pointer) = scale_above_precision(&column.logical_type, &pointer) {
            return Err(RuleViolation::new(
                pointer,
                format!(
                    "column `{}` of relation `{}` has a decimal scale above its precision",
                    column.name, relation.source
                ),
            ));
        }
    }
    Ok(())
}

/// The pointer of the `scale` of a decimal whose scale exceeds its precision, if any.
fn scale_above_precision(logical_type: &LogicalType, pointer: &str) -> Option<String> {
    match logical_type {
        LogicalType::Decimal { precision, scale } if scale > precision => {
            Some(format!("{pointer}/scale"))
        }
        LogicalType::List { element } => {
            scale_above_precision(element, &format!("{pointer}/element"))
        }
        _ => None,
    }
}
