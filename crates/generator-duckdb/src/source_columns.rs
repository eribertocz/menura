//! `Scan` columns that name a catalog column of the dataset `source` rather than a field: the
//! source columns a development-line dataset metric reads as `source.column` (E0D-3, issue
//! #264).

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::{Column, RelationSchema};

use crate::identifiers::quote;

/// The prefix of the name of a `Scan` column that names a catalog column; the rest of the name
/// is the column's name as the reference spells it after normalization (E0D-1.1).
const SOURCE_COLUMN_PREFIX: &str = "menura.source.";

/// The catalog column name that the `Scan` column `name` carries, or `None` when `name` is
/// the name of a field.
pub(crate) fn source_column_name(name: &str) -> Option<&str> {
    name.strip_prefix(SOURCE_COLUMN_PREFIX)
}

/// The column of `relation` that the source column name `name` matches (E0D-1.5): the column
/// spelled exactly `name`, otherwise the one whose normalized name is `name`. A delimited
/// reference matches the first way; a regular one, normalized, the second, which CAT-8.2 makes
/// unambiguous.
pub(crate) fn source_catalog_column<'relation>(
    relation: &'relation RelationSchema,
    name: &str,
) -> Option<&'relation Column> {
    relation
        .columns
        .iter()
        .find(|column| column.name == name)
        .or_else(|| {
            relation
                .columns
                .iter()
                .find(|column| normalize_unquoted_identifier(&column.name) == name)
        })
}

/// The only select item of a `Scan` with no columns: a constant, so that the later nodes
/// still count the rows of the source. Its name is the bare prefix, which names no catalog
/// column.
pub(crate) fn empty_scan_item() -> String {
    format!("TRUE AS {}", quote(SOURCE_COLUMN_PREFIX))
}
