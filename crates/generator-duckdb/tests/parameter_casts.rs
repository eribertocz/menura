//! Unit tests of the explicit casts of bound parameters whose type `DuckDB` would otherwise
//! choose through overload resolution (E6-4.10, E6-4.2, E0F-2.3).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{LogicalType, TimestampPrecision};
use menura_generator_duckdb::GeneratedSql;
use support::{
    big_integer, expression_sql, from_model, from_query, instant, query_with_reference_time,
};

/// The SQL of `expression`, a model expression of type `logical_type`.
#[cfg(test)]
fn model_sql(expression: &str, logical_type: LogicalType) -> GeneratedSql {
    expression_sql(
        expression,
        logical_type,
        from_model("/semantic_model/0/datasets/0/fields/0"),
        &query_with_reference_time(),
    )
    .expect("SQL")
}

/// The SQL of `expression`, a query expression of type `logical_type`: its literals are bound.
#[cfg(test)]
fn query_sql(expression: &str, logical_type: LogicalType) -> GeneratedSql {
    expression_sql(
        expression,
        logical_type,
        from_query("/query/filters/0"),
        &query_with_reference_time(),
    )
    .expect("SQL")
}

fn timestamp() -> LogicalType {
    LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone: false,
    }
}

#[test]
fn e6_4_10_current_timestamp_casts_the_reference_time_before_converting_it() {
    let generated = model_sql("CURRENT_TIMESTAMP", timestamp());
    assert!(
        generated
            .sql
            .contains("CAST(CAST($1 AS TIMESTAMP WITH TIME ZONE) AT TIME ZONE $2 AS TIMESTAMP)")
    );
    assert_eq!(generated.parameters[0].logical_type, instant());
}

#[test]
fn e6_4_10_current_time_casts_the_reference_time_before_converting_it() {
    let generated = model_sql("CURRENT_TIME", LogicalType::Time);
    assert!(
        generated
            .sql
            .contains("CAST(CAST($1 AS TIMESTAMP WITH TIME ZONE) AT TIME ZONE $2 AS TIME)")
    );
}

#[test]
fn e6_4_10_every_use_of_the_reference_time_is_cast() {
    let generated = model_sql(
        "CASE WHEN SHIPPED_AT <= CURRENT_TIMESTAMP THEN CURRENT_TIME END",
        LogicalType::Time,
    );
    assert_eq!(
        generated
            .sql
            .matches("CAST($1 AS TIMESTAMP WITH TIME ZONE) AT TIME ZONE $2")
            .count(),
        2
    );
    assert!(!generated.sql.contains("($1 AT TIME ZONE"));
    assert!(!generated.sql.contains(" $1 AT TIME ZONE"));
    assert_eq!(generated.parameters.len(), 2);
}

#[test]
fn e6_4_10_the_reference_date_is_cast_where_it_is_used() {
    let generated = model_sql("DATEDIFF(week, ORDERED_ON, CURRENT_DATE)", big_integer());
    assert!(
        generated
            .sql
            .contains("CAST(date_trunc('week', CAST($1 AS DATE)) AS DATE)")
    );
}

#[test]
fn e6_4_10_a_date_parameter_is_cast_where_it_is_used() {
    let generated = query_sql("YEAR(DATE '2026-01-01')", big_integer());
    assert!(
        generated
            .sql
            .contains("date_part('year', CAST($1 AS DATE))")
    );
    assert_eq!(generated.parameters[0].logical_type, LogicalType::Date);
}

#[test]
fn e6_4_10_a_timestamp_parameter_is_cast_where_it_is_used() {
    let generated = query_sql("HOUR(TIMESTAMP '2026-01-01 10:00:00')", big_integer());
    assert!(
        generated
            .sql
            .contains("date_part('hour', CAST($1 AS TIMESTAMP))")
    );
}

#[test]
fn e6_4_10_a_date_bound_of_a_timestamp_keeps_the_e6_4_2b_form() {
    let generated = query_sql("SHIPPED_AT >= DATE '2026-01-01'", LogicalType::Boolean);
    assert!(
        generated
            .sql
            .contains(r#""SHIPPED_AT" >= CAST($1 AS TIMESTAMP)"#)
    );
    assert!(!generated.sql.contains("AS DATE"));
}
