//! One expression of `ORDERS`, generated alone: the subject of the function tests (E0F-10).

use menura_contracts::types::common::LogicalType;
use menura_generator_clickhouse::{GenerateError, GeneratedSql};
use menura_model::expression::{DateValue, TimeOfDay, TimestampValue};
use menura_representations::bound_query::{BoundQuery, BoundQueryParts};
use menura_representations::columns::Columns;
use menura_representations::origin::Origin;
use menura_representations::time::{QueryTime, ReferenceTime, TimeZoneName, UtcOffset};

use super::{column, dimension, generate, named, project, scan};

/// The fields of `ORDERS` the expressions read.
const FIELDS: &[&str] = &[
    "AMOUNT",
    "CREATED_AT",
    "ORDERED_ON",
    "QUANTITY",
    "REGION",
    "SHIPPED_AT",
];

/// The SQL of the `Project` of `ORDERS`, from `origin`, whose only column `X` is `expression`
/// of type `logical_type`, for `query`.
pub fn expression_sql(
    expression: &str,
    logical_type: LogicalType,
    origin: Origin,
    query: &BoundQuery,
) -> Result<GeneratedSql, GenerateError> {
    let plan = project(
        scan("ORDERS", FIELDS),
        vec![named("X", expression)],
        vec![column("X", logical_type, true)],
        origin,
    );
    generate(&plan, query)
}

/// The query of the single column `X`, in `America/New_York`, at the reference time
/// `2026-11-15T23:30:00.25-04:00`, whose date in that zone is 2026-11-15.
pub fn query_with_reference_time() -> BoundQuery {
    reference_time_query(&["X"])
}

/// The query of the dimensions `names` at the reference time of `query_with_reference_time`.
pub fn reference_time_query(names: &[&str]) -> BoundQuery {
    let local = TimestampValue {
        date: DateValue::new(2026, 11, 15).expect("a date"),
        time: TimeOfDay::new(23, 30, 0, "25").expect("a time of day"),
    };
    let offset = UtcOffset::from_minutes(-240).expect("an offset");
    let time = QueryTime {
        time_zone: TimeZoneName::new("America/New_York"),
        reference_date: Some(local.date),
        reference_time: Some(ReferenceTime { local, offset }),
    };
    dimensions_query(names, time)
}

/// The query of the single column `X`, in `Europe/Madrid`, without a reference time.
pub fn query_without_reference_time() -> BoundQuery {
    let time = QueryTime {
        time_zone: TimeZoneName::new("Europe/Madrid"),
        reference_date: None,
        reference_time: None,
    };
    dimensions_query(&["X"], time)
}

fn dimensions_query(names: &[&str], time: QueryTime) -> BoundQuery {
    let dimensions = names.iter().map(|name| dimension(name)).collect();
    let columns = Columns::dimensions_then_metrics(dimensions, Vec::new());
    BoundQuery::new(columns, time, BoundQueryParts::default())
}
