//! `Scan` columns that name a catalog column of the dataset `source` rather than a field: the
//! source columns a development-line dataset metric reads as `source.column` (E0D-3, issue
//! #264).

use crate::identifiers::quote;

/// The prefix of the name of a `Scan` column that names a catalog column; the rest of the name
/// is the column's name as the reference spells it after normalization (E0D-1.1).
const SOURCE_COLUMN_PREFIX: &str = "menura.source.";

/// The catalog column name that the `Scan` column `name` carries, or `None` when `name` is
/// the name of a field.
pub(crate) fn source_column_name(name: &str) -> Option<&str> {
    name.strip_prefix(SOURCE_COLUMN_PREFIX)
}

/// The only select item of a `Scan` with no columns: a constant, so that the later nodes
/// still count the rows of the source. Its name is the bare prefix, which names no catalog
/// column.
pub(crate) fn empty_scan_item() -> String {
    format!("TRUE AS {}", quote(SOURCE_COLUMN_PREFIX))
}
