//! The JSON Pointers of the query elements E1 locates (E1-2.2, E1-5 "Origin").

/// `time_zone`.
pub(crate) const TIME_ZONE: &str = "/time_zone";

/// `sql`, the SQL form.
pub(crate) const SQL: &str = "/sql";

/// `query`, the wide-table form.
pub(crate) const QUERY: &str = "/query";

/// `query.dimensions`.
pub(crate) const DIMENSIONS: &str = "/query/dimensions";

/// `query.metrics`.
pub(crate) const METRICS: &str = "/query/metrics";

/// `query.filter`.
pub(crate) const FILTER: &str = "/query/filter";

/// `query.nonportable_extensions`.
pub(crate) const EXTENSIONS: &str = "/query/nonportable_extensions";

/// The pointer of the item at `index` of the array at `array`.
pub(crate) fn item(array: &str, index: usize) -> String {
    format!("{array}/{index}")
}

/// The pointer of the member `member` of the compiler extension data at `index`.
pub(crate) fn extension_member(index: usize, member: &str) -> String {
    format!("{EXTENSIONS}/{index}/data/{member}")
}
