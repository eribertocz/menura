//! Unit tests of the date and time functions (E0F-10.1) and of `CURRENT_*` (E0F-2.3, E6-4.2).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{LogicalType, TimestampPrecision};
use menura_generator_duckdb::{GeneratedSql, InvariantViolation};
use serde_json::json;
use support::{
    big_integer, expression_sql, from_model, from_query, instant, query_with_reference_time,
    query_without_reference_time,
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

fn timestamp() -> LogicalType {
    LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone: false,
    }
}

#[test]
fn e0f_10_1_parts_are_64_bit_integers() {
    let generated = model_sql("YEAR(ORDERED_ON)", big_integer());
    assert!(
        generated
            .sql
            .contains(r#"CAST(date_part('year', "ORDERED_ON") AS BIGINT)"#)
    );
}

#[test]
fn e0f_10_1_week_and_day_of_week_are_iso() {
    let week = model_sql("EXTRACT(WEEK FROM ORDERED_ON)", big_integer());
    assert!(week.sql.contains(r#"date_part('week', "ORDERED_ON")"#));
    let day = model_sql("DATE_PART('dayofweek', ORDERED_ON)", big_integer());
    assert!(day.sql.contains(r#"date_part('isodow', "ORDERED_ON")"#));
    let year_day = model_sql("DAYOFYEAR(ORDERED_ON)", big_integer());
    assert!(year_day.sql.contains(r#"date_part('doy', "ORDERED_ON")"#));
}

#[test]
fn e0f_10_1_second_has_no_fraction() {
    let generated = model_sql("SECOND(SHIPPED_AT)", big_integer());
    assert!(
        generated
            .sql
            .contains(r#"CAST(date_part('second', "SHIPPED_AT") AS BIGINT)"#)
    );
}

#[test]
fn e0f_10_1_parts_of_an_instant_are_read_in_the_query_time_zone() {
    let generated = model_sql("HOUR(CREATED_AT)", big_integer());
    assert!(
        generated
            .sql
            .contains(r#"date_part('hour', ("CREATED_AT" AT TIME ZONE $1))"#)
    );
    assert_eq!(generated.parameters.len(), 1);
    assert_eq!(generated.parameters[0].value, json!("America/New_York"));
}

#[test]
fn e0f_10_1_dateadd_adds_calendar_intervals() {
    let months = model_sql("DATEADD(month, 1, ORDERED_ON)", LogicalType::Date);
    assert!(
        months
            .sql
            .contains(r#"CAST("ORDERED_ON" + to_months(CAST(1 AS INTEGER)) AS DATE)"#)
    );
    let quarters = model_sql("DATEADD('quarter', 2, ORDERED_ON)", LogicalType::Date);
    assert!(quarters.sql.contains("to_months(CAST(2 AS INTEGER) * 3)"));
    let seconds = model_sql("DATEADD(second, QUANTITY, SHIPPED_AT)", timestamp());
    assert!(
        seconds.sql.contains(
            r#"CAST("SHIPPED_AT" + to_seconds(CAST("QUANTITY" AS DOUBLE)) AS TIMESTAMP)"#
        )
    );
}

#[test]
fn e0f_10_1_dateadd_of_an_instant_adds_in_the_query_time_zone() {
    let generated = model_sql("DATEADD(day, 1, CREATED_AT)", instant());
    assert!(generated.sql.contains(
        r#"CAST((("CREATED_AT" AT TIME ZONE $1) + to_days(CAST(1 AS INTEGER))) AT TIME ZONE $1 AS TIMESTAMP WITH TIME ZONE)"#
    ));
}

#[test]
fn e0f_10_1_datediff_counts_calendar_boundaries() {
    let years = model_sql(
        "DATEDIFF(year, ORDERED_ON, DATE '2026-11-04')",
        big_integer(),
    );
    assert!(years.sql.contains(
        r#"CAST(date_part('year', DATE '2026-11-04') - date_part('year', "ORDERED_ON") AS BIGINT)"#
    ));
    let months = model_sql(
        "DATEDIFF(month, ORDERED_ON, DATE '2026-11-04')",
        big_integer(),
    );
    assert!(months.sql.contains(
        r#"(date_part('year', DATE '2026-11-04') * 12 + date_part('month', DATE '2026-11-04')) - (date_part('year', "ORDERED_ON") * 12 + date_part('month', "ORDERED_ON"))"#
    ));
}

#[test]
fn e0f_10_1_datediff_weeks_start_on_monday() {
    let generated = model_sql(
        "DATEDIFF(week, ORDERED_ON, DATE '2026-11-04')",
        big_integer(),
    );
    assert!(generated.sql.contains(
        r#"(CAST(date_trunc('week', DATE '2026-11-04') AS DATE) - CAST(date_trunc('week', "ORDERED_ON") AS DATE)) // 7"#
    ));
}

#[test]
fn e0f_10_1_datediff_of_time_units_counts_truncated_instants() {
    let generated = model_sql("DATEDIFF(hour, CREATED_AT, CREATED_AT)", big_integer());
    assert!(generated.sql.contains(
        r#"(epoch_us(date_trunc('hour', ("CREATED_AT" AT TIME ZONE $1))) - epoch_us(date_trunc('hour', ("CREATED_AT" AT TIME ZONE $1)))) // 3600000000"#
    ));
}

#[test]
fn e6_4_2_datediff_binds_its_arguments_in_order_of_appearance() {
    let generated = expression_sql(
        "DATEDIFF(day, DATE '2026-01-01', ORDERED_ON)",
        big_integer(),
        from_query("/query/filters/0"),
        &query_with_reference_time(),
    )
    .expect("SQL");
    assert!(generated.sql.contains(
        r#"CAST(CAST("ORDERED_ON" AS DATE) - CAST(CAST($1 AS DATE) AS DATE) AS BIGINT)"#
    ));
    assert_eq!(generated.parameters[0].value, json!("2026-01-01"));
}

#[test]
fn e0f_10_1_to_date_and_to_timestamp_are_casts() {
    let date = model_sql("TO_DATE(REGION)", LogicalType::Date);
    assert!(date.sql.contains(r#"CAST("REGION" AS DATE)"#));
    let moment = model_sql("TO_TIMESTAMP(REGION)", timestamp());
    assert!(moment.sql.contains(r#"CAST("REGION" AS TIMESTAMP)"#));
}

#[test]
fn e0f_2_3_current_date_is_the_reference_date_bound_once() {
    let generated = model_sql(
        "CASE WHEN ORDERED_ON <= CURRENT_DATE THEN CURRENT_DATE END",
        LogicalType::Date,
    );
    assert!(
        generated
            .sql
            .contains(r#"CASE WHEN ("ORDERED_ON" <= CAST($1 AS DATE)) THEN CAST($1 AS DATE) END"#)
    );
    assert_eq!(generated.parameters.len(), 1);
    assert_eq!(generated.parameters[0].logical_type, LogicalType::Date);
    assert_eq!(generated.parameters[0].value, json!("2026-11-15"));
}

#[test]
fn e0f_2_3_current_timestamp_is_the_wall_clock_of_the_reference_time() {
    let generated = model_sql("CURRENT_TIMESTAMP", timestamp());
    assert!(
        generated
            .sql
            .contains("CAST(CAST($1 AS TIMESTAMP WITH TIME ZONE) AT TIME ZONE $2 AS TIMESTAMP)")
    );
    assert_eq!(generated.parameters[0].logical_type, instant());
    assert_eq!(
        generated.parameters[0].value,
        json!("2026-11-16T03:30:00.25Z")
    );
    assert_eq!(generated.parameters[1].value, json!("America/New_York"));
}

#[test]
fn e0f_2_3_current_time_is_the_time_of_the_reference_time() {
    let generated = model_sql("CURRENT_TIME", LogicalType::Time);
    assert!(
        generated
            .sql
            .contains("CAST(CAST($1 AS TIMESTAMP WITH TIME ZONE) AT TIME ZONE $2 AS TIME)")
    );
}

#[test]
fn e0f_2_3_current_date_without_reference_time_is_an_invariant_violation() {
    let error = expression_sql(
        "CURRENT_DATE",
        LogicalType::Date,
        from_query("/query/filters/0"),
        &query_without_reference_time(),
    )
    .expect_err("no reference time");
    assert_eq!(error.violation, InvariantViolation::MissingReferenceTime);
}
