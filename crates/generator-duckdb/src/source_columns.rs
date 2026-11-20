//! `Scan` columns that name a catalog column of the dataset `source` rather than a field: the
//! source columns a development-line dataset metric reads as `source.column` (E0D-3, issue
//! #264).

use menura_contracts::types::catalog::RelationSchema;
use menura_model::internal_model::Dataset;

use crate::expression::catalog_column_named;
use crate::identifiers::quote;

/// The shortest `menura.source` followed by one or more `.` that no field name of `dataset`
/// starts with: the prefix of every `Scan` column of `dataset` that names a catalog column, as
/// E4 names it, so that it never collides with a field.
pub(crate) fn source_prefix(dataset: &Dataset) -> String {
    let mut prefix = String::from("menura.source.");
    while dataset
        .fields
        .iter()
        .any(|field| field.name.as_str().starts_with(&prefix))
    {
        prefix.push('.');
    }
    prefix
}

/// The catalog column name that the `Scan` column `name` carries under `prefix`, or `None`
/// when `name` is the name of a field.
pub(crate) fn source_column_name<'name>(prefix: &str, name: &'name str) -> Option<&'name str> {
    name.strip_prefix(prefix)
}

/// The select item of the `Scan` column `name` that names the column `catalog_name` of
/// `relation`: that column in the catalog's spelling (PLN-2.2), or `None` when `relation` has no
/// such column.
pub(crate) fn source_column_item(
    relation: &RelationSchema,
    catalog_name: &str,
    name: &str,
) -> Option<String> {
    let column = catalog_column_named(relation, catalog_name)?;
    Some(format!("{} AS {}", quote(&column.name), quote(name)))
}

/// The only select item of a `Scan` with no columns: a constant, so that the later nodes
/// still count the rows of the source. Its name is the bare `prefix`, which names no catalog
/// column and no field.
pub(crate) fn empty_scan_item(prefix: &str) -> String {
    format!("TRUE AS {}", quote(prefix))
}
