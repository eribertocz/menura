//! Unit tests of the SQL form (E1-4), named after their rule IDs, and their helpers.

mod check_order;
mod subset;

use menura_representations::columns::OutputColumn;
use menura_representations::query_tree::{QueryTree, ReferencePart};
use menura_representations::time::{QueryTime, TimeZoneName};

use super::sql_tree;
use crate::error::{ParseError, ParseErrorCode};

pub(super) fn time() -> QueryTime {
    QueryTime {
        time_zone: TimeZoneName::new("UTC"),
        reference_time: None,
        reference_date: None,
    }
}

pub(super) fn tree(sql: &str) -> QueryTree {
    match sql_tree(sql, time()) {
        Ok(tree) => tree,
        Err(error) => panic!("`{sql}` should parse: {error}"),
    }
}

pub(super) fn error(sql: &str) -> ParseError {
    match sql_tree(sql, time()) {
        Ok(_) => panic!("`{sql}` should be rejected"),
        Err(error) => error,
    }
}

pub(super) fn assert_eqry_05(sql: &str) {
    let error = error(sql);
    assert_eq!(error.code, ParseErrorCode::Eqry05, "`{sql}`: {error}");
    assert_eq!(error.pointer, "/sql", "`{sql}`");
}

pub(super) fn output_names(tree: &QueryTree) -> Vec<String> {
    tree.columns()
        .iter()
        .map(|column| match column {
            OutputColumn::Dimension(dimension) => dimension.output_name.as_str().to_owned(),
            OutputColumn::Metric(metric) => metric.output_name.as_str().to_owned(),
        })
        .collect()
}

pub(super) fn regular(text: &str) -> ReferencePart {
    ReferencePart::Regular(text.to_owned())
}
