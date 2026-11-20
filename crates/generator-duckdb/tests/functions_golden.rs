//! Golden SQL of the E0F-10 functions, one file per family, as the query nodes write them: every
//! query value a bound parameter (E6-4.2), every result cast to its type (E6-4.10).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::LogicalType;
use support::{
    assert_golden, big_integer, column, from_query, generate, named, project, reference_time_query,
    scan,
};

const FIELDS: &[&str] = &["AMOUNT", "CREATED_AT", "ORDERED_ON", "QUANTITY", "REGION"];

/// The SQL of a query `Project` of `ORDERS` computing `columns`: name, expression, type.
#[cfg(test)]
fn golden_sql(columns: &[(&str, &str, LogicalType)]) -> String {
    let plan = project(
        scan("ORDERS", FIELDS),
        columns
            .iter()
            .map(|(name, expression, _)| named(name, expression))
            .collect(),
        columns
            .iter()
            .map(|(name, _, logical_type)| column(name, logical_type.clone(), true))
            .collect(),
        from_query("/query"),
    );
    let names: Vec<&str> = columns.iter().map(|(name, _, _)| *name).collect();
    generate(&plan, &reference_time_query(&names))
        .expect("SQL")
        .sql
}

#[test]
fn e0f_10_1_date_and_time_functions() {
    let sql = golden_sql(&[
        ("WEEK", "EXTRACT(WEEK FROM CREATED_AT)", big_integer()),
        ("LATER", "DATEADD(month, 1, ORDERED_ON)", LogicalType::Date),
        (
            "WEEKS",
            "DATEDIFF(week, ORDERED_ON, CURRENT_DATE)",
            big_integer(),
        ),
        (
            "NOW",
            "CURRENT_TIMESTAMP",
            LogicalType::Timestamp {
                precision: menura_contracts::types::common::TimestampPrecision::Us,
                with_time_zone: false,
            },
        ),
    ]);
    assert_golden(&sql, include_str!("golden/e0f_10_1_date_time.sql"));
}

#[test]
fn e0f_10_2_text_functions() {
    let sql = golden_sql(&[
        ("JOINED", "CONCAT(REGION, '-', REGION)", LogicalType::Text),
        ("PART", "SUBSTRING(REGION, 2, 3)", LogicalType::Text),
        ("FOUND", "POSITION('o' IN REGION)", big_integer()),
        ("MATCHES", "REGION LIKE 'n%'", LogicalType::Boolean),
    ]);
    assert_golden(&sql, include_str!("golden/e0f_10_2_text.sql"));
}

#[test]
fn e0f_10_3_numeric_and_conditional_functions() {
    let sql = golden_sql(&[
        (
            "ROUNDED",
            "ROUND(AMOUNT, 1)",
            LogicalType::Decimal {
                precision: 10,
                scale: 1,
            },
        ),
        (
            "REMAINDER",
            "MOD(QUANTITY, 3)",
            LogicalType::Integer {
                bits: menura_contracts::types::common::IntegerBits::OneHundredTwentyEight,
                signed: true,
            },
        ),
        ("FILLED", "COALESCE(QUANTITY, 0)", big_integer()),
    ]);
    assert_golden(&sql, include_str!("golden/e0f_10_3_numbers.sql"));
}
