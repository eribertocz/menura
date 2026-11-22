//! The E1 stage function: envelope, JSON Schema, then the tree of the SQL form, or the filter
//! text, empty query and tree of the wide-table form, in the order of E2-5 rows 0b to 2.

use menura_contracts::types::query::{QueryDocument, QueryForm, WideTableQuery};
use menura_contracts::{IncomingContract, decode};
use menura_representations::query_tree::QueryTree;
use menura_representations::time::{QueryTime, TimeZoneName};

use crate::envelope::check_envelope;
use crate::error::{ParseError, ParseErrorCode, ParseWarning};
use crate::filters::parse_filters;
use crate::pointer::QUERY;
use crate::reference_time::read_reference_time;
use crate::sql::sql_tree;
use crate::time_zone::local_date;
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
/// `EQRY-01`, `EQRY-02`, `EQRY-06`, the JSON Schema with the range of `reference_time`
/// (`E_INVALID_QUERY`, QRY-2), the filter text
/// (`E0F-01`, `E0F-02`) and `E_EMPTY_AGGREGATION_QUERY` for the wide-table form, or the SQL
/// text (`EQRY-05`, `E0F-02`, `EQRY-13`, E1-4.11) for the SQL form. An error after the JSON Schema carries the `WQRY-01` warnings
/// of the query (E1-2.4), which E1 reports for every query that passes the schema.
pub fn parse_query(text: &str) -> Result<ParsedQuery, ParseError> {
    let contract = IncomingContract::Query;
    let document = contract.parse_json(text)?;
    check_envelope(&document)?;
    contract.validate(&document)?;
    let query: QueryDocument = decode(contract, &document)?;
    let time = query_time(&query)?;
    let warnings = former_name_warnings(&query);
    match query_tree(&query, time) {
        Ok(tree) => Ok(ParsedQuery { tree, warnings }),
        Err(error) => Err(error.with_warnings(warnings)),
    }
}

/// The query tree of `query`, which passed the envelope and the JSON Schema: the SQL text of
/// the SQL form (E1-4), or the filter text then the empty query of the wide-table form.
fn query_tree(query: &QueryDocument, time: QueryTime) -> Result<QueryTree, ParseError> {
    let wide_table = match &query.form {
        QueryForm::WideTable(wide_table) => wide_table,
        QueryForm::Sql(sql) => return sql_tree(sql, time),
    };
    let filters = parse_filters(wide_table)?;
    if is_empty_aggregation(wide_table) {
        return Err(ParseError::new(
            ParseErrorCode::EmptyAggregationQuery,
            QUERY,
            "the query has no dimensions and no metrics; request at least one".to_owned(),
        ));
    }
    wide_table_tree(wide_table, time, filters)
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
                    format!(
                        "the reference time `{text}` has a year outside 0001-9999, as written \
                         or in UTC"
                    ),
                )
            })
        })
        .transpose()?;
    let reference_date = reference_time
        .as_ref()
        .and_then(|reference| local_date(reference, &query.time_zone));
    Ok(QueryTime {
        time_zone: TimeZoneName::new(&query.time_zone),
        reference_time,
        reference_date,
    })
}
