//! Bound queries of the SQL form (E1-4): output columns in `SELECT` order, located in `/sql`.

use menura_representations::bound_query::{
    BoundDimension, BoundDimensionKind, BoundMetric, BoundMetricKind, BoundQuery, BoundQueryParts,
    MetricPath,
};
use menura_representations::columns::{Columns, OutputColumn};
use menura_representations::names::OutputName;
use menura_representations::origin::{QueryLocation, TextPosition};
use menura_representations::time::{QueryTime, TimeZoneName};

use super::{name, tree};

/// An output column of the SQL form.
pub type SqlColumn = OutputColumn<BoundDimension, BoundMetric>;

/// The location of the `SELECT` item that starts at `column` of the first line of the SQL text.
pub fn select_item(column: usize) -> QueryLocation {
    QueryLocation::new("/sql", TextPosition::new(1, column))
}

/// The dimension `expression AS output`, over fields of `ORDERS` written as E2 writes them
/// (`"ORDERS"."FIELD"`), at `column`.
pub fn expression_dimension(expression: &str, output: &str, column: usize) -> SqlColumn {
    OutputColumn::Dimension(BoundDimension {
        kind: BoundDimensionKind::Expression {
            dataset: name("ORDERS"),
            expression: tree(expression),
        },
        output_name: OutputName::new(output),
        origin: select_item(column),
    })
}

/// The metric `AGG(metric_name) AS output` of the semantic model, at `column`.
pub fn aggregated(metric_name: &str, output: &str, column: usize) -> SqlColumn {
    OutputColumn::Metric(BoundMetric {
        kind: BoundMetricKind::Reference(MetricPath::SemanticModel(name(metric_name))),
        output_name: OutputName::new(output),
        origin: select_item(column),
    })
}

/// The SQL-form query with `columns` in `SELECT` order and `parts`, in time zone `UTC`.
pub fn sql_query(columns: Vec<SqlColumn>, parts: BoundQueryParts) -> BoundQuery {
    let time = QueryTime {
        time_zone: TimeZoneName::new("UTC"),
        reference_time: None,
        reference_date: None,
    };
    BoundQuery::new(Columns::in_select_order(columns), time, parts)
}
