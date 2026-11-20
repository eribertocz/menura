//! The E1 stage function: envelope, JSON Schema, filter text, SQL form, empty query, tree, in
//! the order of E2-5 rows 0b to 2.

use menura_contracts::types::query::{QueryDocument, QueryForm, WideTableQuery};
use menura_contracts::{IncomingContract, decode};
use menura_representations::query_tree::QueryTree;
use menura_representations::time::{QueryTime, TimeZoneName};

use crate::envelope::check_envelope;
use crate::error::{ParseError, ParseErrorCode, ParseWarning};
use crate::filters::parse_filters;
use crate::pointer::{QUERY, SQL};
use crate::reference_time::read_reference_time;
use crate::tree::wide_table_tree;
use crate::warnings::former_name_warnings;

/// A query E1 accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedQuery {
    /// The query tree (E1-5).
    pub tree: QueryTree,
    /// The `WQRY-01` warnings (E1-2.4), in document order.
    pub warnings: Vec<ParseWarning>,
}

/// Parses the text of a query document into its query tree (E1-1).
///
/// E1 checks the envelope and the shape only; every check that needs the model runs in E2.
///
/// ```
/// use menura_e1_parse::parse_query;
/// let parsed = parse_query(r#"{
///     "contract": "menura.query", "contract_version": "1.1.0", "time_zone": "UTC",
///     "query": {"dimensions": ["orders.region"], "metrics": ["revenue"]}
/// }"#);
/// assert_eq!(parsed.map(|parsed| parsed.tree.columns().iter().count()).ok(), Some(2));
/// ```
///
/// # Errors
///
/// The first error, in this order (E2-5 rows 0b to 2): not JSON (`E_INVALID_QUERY`), `EQRY-08`,
/// `EQRY-01`, `EQRY-02`, `EQRY-06`, the JSON Schema (`E_INVALID_QUERY`), the filter text
/// (`E0F-01`, `E0F-02`), the SQL form (`E_NOT_YET_SUPPORTED`, VS-2), then
/// `E_EMPTY_AGGREGATION_QUERY`.
pub fn parse_query(text: &str) -> Result<ParsedQuery, ParseError> {
    let contract = IncomingContract::Query;
    let document = contract.parse_json(text)?;
    check_envelope(&document)?;
    contract.validate(&document)?;
    let query: QueryDocument = decode(contract, &document)?;
    let wide_table = match &query.form {
        QueryForm::WideTable(wide_table) => wide_table,
        QueryForm::Sql(_) => return Err(sql_form_not_yet_supported()),
    };
    let filters = parse_filters(wide_table)?;
    if is_empty_aggregation(wide_table) {
        return Err(ParseError::new(
            ParseErrorCode::EmptyAggregationQuery,
            QUERY,
            "the query has no dimensions and no metrics; request at least one".to_owned(),
        ));
    }
    let tree = wide_table_tree(wide_table, query_time(&query)?, filters)?;
    Ok(ParsedQuery {
        tree,
        warnings: former_name_warnings(&query),
    })
}

/// VS-2: no SQL text is parsed before the version that adds the SQL form.
fn sql_form_not_yet_supported() -> ParseError {
    ParseError::new(
        ParseErrorCode::NotYetSupported,
        SQL,
        "the SQL form is not supported before 0.5.0; use the wide-table form (`query`)".to_owned(),
    )
}

fn is_empty_aggregation(query: &WideTableQuery) -> bool {
    query.dimensions.as_ref().is_none_or(Vec::is_empty)
        && query.metrics.as_ref().is_none_or(Vec::is_empty)
}

fn query_time(query: &QueryDocument) -> Result<QueryTime, ParseError> {
    let reference_time = query
        .reference_time
        .as_deref()
        .map(|text| {
            read_reference_time(text).ok_or_else(|| {
                ParseError::new(
                    ParseErrorCode::InvalidQuery,
                    "/reference_time",
                    format!("the reference time `{text}` cannot be represented"),
                )
            })
        })
        .transpose()?;
    Ok(QueryTime {
        time_zone: TimeZoneName::new(&query.time_zone),
        reference_time,
    })
}
