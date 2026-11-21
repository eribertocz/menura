//! The `Project` columns that compute a derived formula: their divisions follow the
//! `on_division_by_zero` of their derived metric (DER-3.1, E6-4.6, E6-4.6a).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::common::{FloatBits, LogicalType};
use menura_model::internal_model::{DivisionByZero, NormalizedName};
use menura_representations::logical_plan::{DerivedFormula, NamedExpression, PlanNode};
use support::{column, named, project, scan};
use support::{from_model, generate_all};

fn double() -> LogicalType {
    LogicalType::Float {
        bits: FloatBits::SixtyFour,
    }
}

fn formula(name: &str, expression: &str, rule: Option<(&str, DivisionByZero)>) -> NamedExpression {
    NamedExpression {
        derived: rule.map(|(metric, on_division_by_zero)| DerivedFormula {
            metric: NormalizedName::new(metric),
            on_division_by_zero,
        }),
        ..named(name, expression)
    }
}

/// The `Project` over the `Scan` of `ORDERS` that computes `expressions`, each a float.
fn ratios(expressions: Vec<NamedExpression>) -> PlanNode {
    let schema = expressions
        .iter()
        .map(|named| column(named.name.as_str(), double(), true))
        .collect();
    project(
        scan("ORDERS", &["AMOUNT", "QUANTITY"]),
        expressions,
        schema,
        from_model("/semantic_model/0/metrics/0"),
    )
}

#[test]
fn e6_4_6a_a_derived_formula_with_error_raises_rder_01_naming_its_metric() {
    let plan = ratios(vec![formula(
        "STRICT_RATE",
        "AMOUNT / QUANTITY",
        Some(("STRICT_RATE", DivisionByZero::Error)),
    )]);

    let sql = generate_all(&plan).sql;

    assert!(
        sql.contains("THEN error('RDER-01: division by zero in metric STRICT_RATE') ELSE"),
        "{sql}"
    );
}

#[test]
fn e6_4_6_a_derived_formula_with_null_yields_null() {
    let plan = ratios(vec![formula(
        "RATE",
        "AMOUNT / QUANTITY",
        Some(("RATE", DivisionByZero::Null)),
    )]);

    let sql = generate_all(&plan).sql;

    assert!(sql.contains("THEN NULL ELSE"), "{sql}");
    assert!(!sql.contains("RDER-01"), "{sql}");
}

#[test]
fn der_3_1_each_column_follows_the_rule_of_its_own_derived_metric() {
    let plan = ratios(vec![
        formula("PLAIN", "AMOUNT / QUANTITY", None),
        formula(
            "STRICT_RATE",
            "QUANTITY / AMOUNT",
            Some(("STRICT_RATE", DivisionByZero::Error)),
        ),
        formula(
            "RATE",
            "AMOUNT / AMOUNT",
            Some(("RATE", DivisionByZero::Null)),
        ),
    ]);

    let sql = generate_all(&plan).sql;

    assert_eq!(sql.matches("RDER-01").count(), 1, "{sql}");
    assert_eq!(sql.matches("THEN NULL ELSE").count(), 2, "{sql}");
    let error = sql.find("RDER-01").expect("the error");
    let strict_column = sql.find(r#"AS "STRICT_RATE""#).expect("the strict column");
    let plain_column = sql.find(r#"AS "PLAIN""#).expect("the plain column");
    assert!(plain_column < error && error < strict_column, "{sql}");
}
