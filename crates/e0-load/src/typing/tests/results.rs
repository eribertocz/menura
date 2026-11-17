//! Result types and nullability: E0F-3, E0F-3.1, E0F-3.2, E0F-5, E0F-5a, E0F-6.1, E0F-7.1 and
//! E0F-10.

use semantic_compiler_contracts::types::common::{FloatBits, LogicalType};

use super::*;
use crate::typing::value_type::{big_integer, decimal, double, huge_integer, integer, timestamp};

fn typed(value_type: LogicalType, nullable: bool) -> Probe {
    Probe::Typed(Typed::value(value_type, nullable))
}

#[test]
fn e0f_7_1_nullability_comes_from_the_catalog() {
    assert_eq!(field("quantity"), typed(integer(), false));
    assert_eq!(field("discount"), typed(double(), true));
    assert_eq!(field("quantity + 1"), typed(huge_integer(), false));
    assert_eq!(field("discount + 1"), typed(double(), true));
    assert_eq!(field("note IS NULL"), typed(LogicalType::Boolean, false));
    assert_eq!(
        field("COALESCE(note, status)"),
        typed(LogicalType::Text, false)
    );
    assert_eq!(
        field("COALESCE(note, NULL)"),
        typed(LogicalType::Text, true)
    );
    assert_eq!(
        field("CASE WHEN quantity > 1 THEN status END"),
        typed(LogicalType::Text, true)
    );
    assert_eq!(
        field("CASE WHEN quantity > 1 THEN status ELSE 'x' END"),
        typed(LogicalType::Text, false)
    );
    assert_eq!(field("NULLIF(status, 'x')"), typed(LogicalType::Text, true));
    assert_eq!(field("ZEROIFNULL(discount)"), typed(double(), false));
    assert_eq!(field("LEFT(status, 2)"), typed(LogicalType::Text, false));
    assert_eq!(
        field("LEFT(status, quantity)"),
        typed(LogicalType::Text, true)
    );
    assert_eq!(field("CAST(note AS INTEGER)"), typed(integer(), true));
    assert_eq!(metric("COUNT(orders.note)"), typed(big_integer(), false));
    assert_eq!(metric("MAX(orders.status)"), typed(LogicalType::Text, true));
}

#[test]
fn e0f_6_1_division_and_modulo_can_be_null() {
    assert_eq!(field("quantity / 2"), typed(double(), true));
    assert_eq!(field("quantity % 2"), typed(huge_integer(), true));
    assert_eq!(field("MOD(quantity, 2)"), typed(huge_integer(), true));
    assert_eq!(field("SQRT(quantity)"), typed(double(), true));
}

#[test]
fn e0f_5a_scalar_result_types() {
    assert_eq!(field("quantity / quantity"), typed(double(), true));
    assert_eq!(
        field("quantity * customer_id"),
        typed(huge_integer(), false)
    );
    assert_eq!(field("price + price"), typed(decimal(11, 2), false));
    assert_eq!(field("price - 1"), typed(decimal(38, 2), false));
    assert_eq!(field("price * price"), typed(decimal(20, 4), false));
    assert_eq!(field("price * 1.5"), typed(decimal(12, 3), false));
    assert_eq!(field("price + discount"), typed(double(), true));
    assert_eq!(field("-quantity"), typed(huge_integer(), false));
    assert_eq!(field("-price"), typed(decimal(10, 2), false));
    assert_eq!(field("ROUND(price, 1)"), typed(decimal(10, 1), false));
    assert_eq!(field("ROUND(price)"), typed(decimal(10, 0), false));
    assert_eq!(field("TRUNC(quantity, -1)"), typed(huge_integer(), false));
    assert_eq!(field("ROUND(discount, 2)"), typed(double(), true));
    assert_eq!(
        field("CASE WHEN quantity > 1 THEN quantity ELSE 0 END"),
        typed(big_integer(), false)
    );
    assert_eq!(field("COALESCE(price, 1.5)"), typed(decimal(10, 2), false));
    assert_eq!(field("12345678901234567890"), typed(decimal(20, 0), false));
    assert_eq!(field(".5"), typed(decimal(1, 1), false));
    assert_eq!(field("1E3"), typed(double(), false));
}

#[test]
fn e0f_3_implicit_conversions_decide_comparisons() {
    assert_eq!(
        field("quantity = customer_id"),
        typed(LogicalType::Boolean, false)
    );
    assert_eq!(
        field("quantity BETWEEN 1 AND 3"),
        typed(LogicalType::Boolean, false)
    );
    assert_eq!(
        field("quantity IN (1, 2, NULL)"),
        typed(LogicalType::Boolean, true)
    );
    assert_eq!(
        field("price > CAST(1 AS DECIMAL(12, 2))"),
        typed(LogicalType::Boolean, false)
    );
    assert_eq!(
        field("ordered_on < DATE '2026-11-02'"),
        typed(LogicalType::Boolean, false)
    );
    assert_eq!(
        field("ordered_at < TIMESTAMP '2026-11-02 00:00:00'"),
        typed(LogicalType::Boolean, true)
    );
    assert_eq!(
        field("status IS DISTINCT FROM note"),
        typed(LogicalType::Boolean, false)
    );
}

#[test]
fn e0f_10_function_result_types() {
    assert_eq!(field("YEAR(ordered_on)"), typed(big_integer(), false));
    assert_eq!(
        field("EXTRACT(HOUR FROM ordered_at)"),
        typed(big_integer(), true)
    );
    assert_eq!(
        field("DATE_TRUNC('month', ordered_on)"),
        typed(LogicalType::Date, false)
    );
    assert_eq!(
        field("DATEADD(hour, 1, ordered_at)"),
        typed(timestamp(), true)
    );
    assert_eq!(
        field("DATEDIFF(day, ordered_on, ordered_on)"),
        typed(big_integer(), false)
    );
    assert_eq!(field("CURRENT_TIMESTAMP"), typed(timestamp(), false));
    assert_eq!(field("TO_DATE(status)"), typed(LogicalType::Date, false));
    assert_eq!(
        field("CONCAT(status, note)"),
        typed(LogicalType::Text, true)
    );
    assert_eq!(
        field("POSITION('a' IN status)"),
        typed(big_integer(), false)
    );
    assert_eq!(
        field("SPLIT_PART(status, '-', 2)"),
        typed(LogicalType::Text, false)
    );
    assert_eq!(
        field("STARTSWITH(status, 'O')"),
        typed(LogicalType::Boolean, false)
    );
    assert_eq!(field("ABS(quantity)"), typed(huge_integer(), false));
    assert_eq!(field("SIGN(price)"), typed(big_integer(), false));
    assert_eq!(field("FLOOR(price)"), typed(decimal(9, 0), false));
    assert_eq!(
        field("GREATEST(quantity, customer_id)"),
        typed(big_integer(), false)
    );
    assert_eq!(
        field("IF(quantity > 1, status, note)"),
        typed(LogicalType::Text, true)
    );
    assert_eq!(
        field("NVL2(note, quantity, 0)"),
        typed(big_integer(), false)
    );
}

#[test]
fn e0f_5_aggregate_result_types() {
    assert_eq!(metric("SUM(orders.quantity)"), typed(huge_integer(), true));
    assert_eq!(
        metric("SUM(DISTINCT orders.price)"),
        typed(decimal(38, 2), true)
    );
    assert_eq!(metric("SUM(orders.discount)"), typed(double(), true));
    assert_eq!(metric("AVG(orders.quantity)"), typed(double(), true));
    assert_eq!(metric("STDDEV(orders.price)"), typed(double(), true));
    assert_eq!(metric("MEDIAN(orders.price)"), typed(double(), true));
    assert_eq!(
        metric("PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY orders.quantity)"),
        typed(double(), true)
    );
    assert_eq!(
        metric("MIN(orders.ordered_on)"),
        typed(LogicalType::Date, true)
    );
    assert_eq!(
        metric("COUNT(DISTINCT orders.status)"),
        typed(big_integer(), false)
    );
    assert_eq!(
        metric("SUM(orders.quantity) / COUNT(*)"),
        typed(double(), true)
    );
    assert_eq!(
        metric("SUM(orders.quantity) FILTER (WHERE orders.status = 'OPEN')"),
        typed(huge_integer(), true)
    );
    let float = LogicalType::Float {
        bits: FloatBits::SixtyFour,
    };
    assert_eq!(metric("MAX(orders.discount)"), typed(float, true));
}

#[test]
fn e0f_3_1_numeric_operands_of_any_kinds_are_comparable() {
    let boolean = |nullable| typed(LogicalType::Boolean, nullable);
    assert_eq!(field("price > 0"), boolean(false));
    assert_eq!(field("discount > 1"), boolean(true));
    assert_eq!(field("price BETWEEN 0 AND discount"), boolean(true));
    assert_eq!(field("quantity IN (1.5, 2E0, NULL)"), boolean(true));
    assert_eq!(field("price IS DISTINCT FROM rate"), boolean(false));
    assert_eq!(field("price = status"), Probe::Problem("E0F-03"));
    assert_eq!(
        field("ordered_on IN (ordered_at)"),
        Probe::Problem("E0F-03")
    );
    assert_eq!(field("NULL = NULL"), boolean(true));
}

#[test]
fn e0f_3_2_null_takes_the_type_of_another_operand() {
    assert_eq!(field("NULL + 1"), typed(huge_integer(), true));
    assert_eq!(field("price * NULL"), typed(decimal(20, 4), true));
    assert_eq!(field("NULL / quantity"), typed(double(), true));
    assert_eq!(
        field("NULLIF(NULL, status)"),
        typed(LogicalType::Text, true)
    );
    assert_eq!(field("COALESCE(NULL, quantity)"), typed(integer(), false));
    assert_eq!(field("SIGN(NULL)"), typed(big_integer(), true));
    assert_eq!(field("LENGTH(NULL)"), typed(big_integer(), true));
    assert_eq!(
        field("CAST(NULL AS INTEGER) + 1"),
        typed(huge_integer(), true)
    );
    assert_eq!(metric("AVG(NULL)"), typed(double(), true));
    assert_eq!(metric("COUNT(NULL)"), typed(big_integer(), false));
}

#[test]
fn e0f_3_2_e0f_03_null_without_another_operand_has_no_type() {
    for expression in [
        "-NULL",
        "NULL + NULL",
        "ABS(NULL)",
        "FLOOR(NULL)",
        "ROUND(NULL)",
        "ZEROIFNULL(NULL)",
        "DATE_TRUNC('day', NULL)",
        "DATEADD(day, 1, NULL)",
        "COALESCE(NULL, NULL)",
        "CASE WHEN quantity > 1 THEN NULL END",
        "NULLIF(NULL, NULL)",
    ] {
        assert_eq!(field(expression), Probe::Problem("E0F-03"), "{expression}");
    }
    assert_eq!(metric("SUM(NULL)"), Probe::Problem("E0F-03"));
    assert_eq!(metric("MIN(NULL)"), Probe::Problem("E0F-03"));
}

#[test]
fn e0f_5_median_and_percentiles_of_a_float() {
    assert_eq!(metric("MEDIAN(orders.discount)"), typed(double(), true));
    assert_eq!(
        metric("PERCENTILE_DISC(0.25) WITHIN GROUP (ORDER BY orders.discount DESC)"),
        typed(double(), true)
    );
}

#[test]
fn e0f_5a_an_integer_counts_as_decimal_38_0_in_a_product() {
    assert_eq!(field("price * quantity"), typed(decimal(38, 2), false));
    assert_eq!(field("quantity * rate"), typed(decimal(38, 20), false));
}

#[test]
fn e0f_5a_e0f_03_round_and_trunc_digits_outside_the_table() {
    assert_eq!(field("ROUND(price, -1)"), Probe::Problem("E0F-03"));
    assert_eq!(field("TRUNC(quantity, 1)"), Probe::Problem("E0F-03"));
    assert_eq!(field("ROUND(quantity, 0)"), typed(huge_integer(), false));
    assert_eq!(field("TRUNC(price, 0)"), typed(decimal(10, 0), false));
}

#[test]
fn e0f_5a_e0f_03_split_part_and_percentile_literals() {
    for expression in [
        "SPLIT_PART(status, '-', quantity)",
        "SPLIT_PART(status, '-', 0)",
        "SPLIT_PART(status, '-', 1 + 1)",
    ] {
        assert_eq!(field(expression), Probe::Problem("E0F-03"), "{expression}");
    }
    assert_eq!(
        field("SPLIT_PART(status, '-', 1)"),
        typed(LogicalType::Text, false)
    );
    for expression in [
        "PERCENTILE_CONT(orders.discount) WITHIN GROUP (ORDER BY orders.quantity)",
        "PERCENTILE_DISC(1.5) WITHIN GROUP (ORDER BY orders.quantity)",
        "PERCENTILE_CONT(2E0) WITHIN GROUP (ORDER BY orders.quantity)",
        "PERCENTILE_CONT(10) WITHIN GROUP (ORDER BY orders.quantity)",
    ] {
        assert_eq!(metric(expression), Probe::Problem("E0F-03"), "{expression}");
    }
    for expression in [
        "PERCENTILE_CONT(0) WITHIN GROUP (ORDER BY orders.quantity)",
        "PERCENTILE_CONT(1) WITHIN GROUP (ORDER BY orders.quantity)",
        "PERCENTILE_CONT(1.000) WITHIN GROUP (ORDER BY orders.quantity)",
        "PERCENTILE_CONT(5E-1) WITHIN GROUP (ORDER BY orders.quantity)",
        "PERCENTILE_CONT(.5) WITHIN GROUP (ORDER BY orders.quantity)",
    ] {
        assert_eq!(metric(expression), typed(double(), true), "{expression}");
    }
}

#[test]
fn not_yet_settled_cases_get_no_type_and_no_diagnostic() {
    assert_eq!(field("CASE price WHEN 1 THEN 'one' END"), Probe::Untyped);
    assert_eq!(field("NULL"), Probe::Untyped);
    assert_eq!(
        field("CASE quantity WHEN 1 THEN 'one' END"),
        typed(LogicalType::Text, true)
    );
}

#[test]
fn elements_reported_elsewhere_get_no_type_and_no_diagnostic() {
    assert_eq!(field("missing_column + 1"), Probe::Untyped);
    assert_eq!(field("orders.quantity"), Probe::Untyped);
    assert_eq!(field("SUM("), Probe::Untyped);
    assert_eq!(metric("SUM(orders.missing)"), Probe::Untyped);
    assert_eq!(metric("SUM(quantity)"), Probe::Untyped);
    assert_eq!(metric("AGG(probe) + 1"), Probe::Untyped);
    assert_eq!(field("USER_ATTRIBUTE('region')"), Probe::Untyped);
}
