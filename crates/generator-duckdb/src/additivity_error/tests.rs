//! Tests of the runtime errors of semi-additive metrics (E6-4.6b, ADD-7).

use menura_model::internal_model::NormalizedName;

use super::AdditivityCondition;

fn raised(condition: AdditivityCondition, metric: &str) -> String {
    condition.raise_sql(&NormalizedName::new(metric))
}

#[test]
fn radd_01_missing_entity_names_the_metric() {
    assert_eq!(
        raised(AdditivityCondition::MissingEntity, "BALANCE"),
        "error('RADD-01: missing: error and an expected entity has no row in metric BALANCE')"
    );
}

#[test]
fn radd_02_unexpected_row_names_the_metric() {
    assert_eq!(
        raised(AdditivityCondition::UnexpectedRow, "BALANCE"),
        "error('RADD-02: unexpected: error and an unexpected row exists in metric BALANCE')"
    );
}

#[test]
fn radd_03_null_value_names_the_metric() {
    assert_eq!(
        raised(AdditivityCondition::NullValue, "BALANCE"),
        "error('RADD-03: nulls: error and a NULL value exists in metric BALANCE')"
    );
}

#[test]
fn radd_04_stale_value_names_the_metric() {
    assert_eq!(
        raised(AdditivityCondition::StaleValue, "BALANCE"),
        "error('RADD-04: on_stale: error and a carried value is too old in metric BALANCE')"
    );
}

#[test]
fn e6_4_11_the_message_is_a_quoted_literal() {
    assert_eq!(
        raised(AdditivityCondition::NullValue, "O'BRIEN STOCK"),
        "error('RADD-03: nulls: error and a NULL value exists in metric O''BRIEN STOCK')"
    );
}
