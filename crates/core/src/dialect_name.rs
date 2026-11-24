//! The names of the dialects this release generates (architecture section 3.5, E6-7), in
//! CON-5 order, as the capability profile lists them (DSC-4.3).

use menura_contracts::types::common::Dialect;

/// Every dialect with its name, in CON-5 order of the names.
pub(crate) const DIALECTS: [(&str, Dialect); 2] = [
    ("clickhouse", Dialect::Clickhouse),
    ("duckdb", Dialect::Duckdb),
];

/// The dialect named `name`; `None` when no dialect has that name.
pub(crate) fn dialect_named(name: &str) -> Option<Dialect> {
    DIALECTS
        .iter()
        .find(|(dialect_name, _)| *dialect_name == name)
        .map(|(_, dialect)| *dialect)
}

/// The names of every dialect, in CON-5 order.
pub(crate) fn dialect_names() -> impl Iterator<Item = &'static str> {
    DIALECTS.iter().map(|(name, _)| *name)
}

/// The names of every dialect, each in backticks, the last joined by `conjunction`: for
/// example ``"`clickhouse` and `duckdb`"`` with `"and"`.
pub(crate) fn quoted_dialect_names(conjunction: &str) -> String {
    let quoted: Vec<String> = dialect_names().map(|name| format!("`{name}`")).collect();
    match quoted.split_last() {
        Some((last, [])) => last.clone(),
        Some((last, rest)) => format!("{} {conjunction} {last}", rest.join(", ")),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn e6_7_each_dialect_is_named() {
        assert_eq!(dialect_named("clickhouse"), Some(Dialect::Clickhouse));
        assert_eq!(dialect_named("duckdb"), Some(Dialect::Duckdb));
        assert_eq!(dialect_named("DuckDB"), None);
    }

    #[test]
    fn e6_7_the_names_are_quoted_and_joined() {
        assert_eq!(quoted_dialect_names("and"), "`clickhouse` and `duckdb`");
        assert_eq!(quoted_dialect_names("or"), "`clickhouse` or `duckdb`");
    }

    #[test]
    fn dsc_4_3_the_names_are_sorted() {
        let names: Vec<&str> = dialect_names().collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted);
    }
}
