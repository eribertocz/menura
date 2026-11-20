//! Filter texts parsed as expressions of the E0F-2 language (E1-3.2).

use menura_contracts::types::query::WideTableQuery;
use menura_expressions::{ExpressionCode, parse};
use menura_representations::origin::QueryLocation;
use menura_representations::query_tree::QueryFilter;

use crate::error::{ParseError, ParseErrorCode};
use crate::pointer::{self, FILTER};

/// Parses `filter`, then each `filters` item in index order, and returns them in that order.
///
/// # Errors
///
/// The first filter text that does not parse (`E0F-01`) or uses a construct outside the subset
/// (`E0F-02`), located at that filter.
pub(crate) fn parse_filters(query: &WideTableQuery) -> Result<Vec<QueryFilter>, ParseError> {
    let extension_filters = query
        .nonportable_extensions
        .iter()
        .flatten()
        .enumerate()
        .flat_map(|(index, extension)| {
            extension
                .data
                .filters
                .iter()
                .flatten()
                .enumerate()
                .map(move |(item, text)| {
                    (
                        pointer::item(&pointer::extension_member(index, "filters"), item),
                        text,
                    )
                })
        });
    query
        .filter
        .iter()
        .map(|text| (FILTER.to_owned(), text))
        .chain(extension_filters)
        .map(|(location, text)| parse_filter(&location, text))
        .collect()
}

fn parse_filter(location: &str, text: &str) -> Result<QueryFilter, ParseError> {
    match parse(text) {
        Ok(expression) => Ok(QueryFilter {
            expression,
            origin: QueryLocation::new(location, None),
        }),
        Err(error) => {
            let code = match error.code() {
                ExpressionCode::Unparsable => ParseErrorCode::E0f01,
                ExpressionCode::OutsideSubset => ParseErrorCode::E0f02,
            };
            Err(ParseError::new(
                code,
                location,
                format!("the filter `{text}` is not accepted: {}", error.kind()),
            ))
        }
    }
}
