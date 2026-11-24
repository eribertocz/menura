//! The `e0-additivity` conformance model and its variants, shared by the tests of the checks
//! of additivity in E0.c, E0.e, E0.f and E0.g.

/// The model document of every `e0-additivity*` conformance model, by name.
const MODELS: [(&str, &str); 10] = [
    (
        "e0-additivity",
        include_str!("../../../../../conformance/models/e0-additivity/model.yaml"),
    ),
    (
        "e0-additivity-non-additive-key-001",
        include_str!(
            "../../../../../conformance/models/e0-additivity-non-additive-key-001/model.yaml"
        ),
    ),
    (
        "e0-additivity-missing-key-001",
        include_str!("../../../../../conformance/models/e0-additivity-missing-key-001/model.yaml"),
    ),
    (
        "e0-additivity-time-kind-001",
        include_str!("../../../../../conformance/models/e0-additivity-time-kind-001/model.yaml"),
    ),
    (
        "e0-additivity-carry-forward-001",
        include_str!(
            "../../../../../conformance/models/e0-additivity-carry-forward-001/model.yaml"
        ),
    ),
    (
        "e0-additivity-selection-001",
        include_str!("../../../../../conformance/models/e0-additivity-selection-001/model.yaml"),
    ),
    (
        "e0-additivity-over-in-entity-001",
        include_str!(
            "../../../../../conformance/models/e0-additivity-over-in-entity-001/model.yaml"
        ),
    ),
    (
        "e0-additivity-unreachable-001",
        include_str!("../../../../../conformance/models/e0-additivity-unreachable-001/model.yaml"),
    ),
    (
        "e0-additivity-population-001",
        include_str!("../../../../../conformance/models/e0-additivity-population-001/model.yaml"),
    ),
    (
        "e0-additivity-conflicts",
        include_str!("../../../../../conformance/models/e0-additivity-conflicts/model.yaml"),
    ),
];

/// The catalog schema of every `e0-additivity*` model: `shop.debt` (`customer_id` 32-bit,
/// `period` date, `balance` 32-bit) and `shop.customers` (`customer_id` 32-bit,
/// `category` text, `valid_from` date, nullable `valid_to` date).
pub(crate) const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-additivity/catalog-schema.json");

/// The model document of the conformance model `name`.
pub(crate) fn conformance_model(name: &str) -> &'static str {
    MODELS
        .iter()
        .find(|(model, _)| *model == name)
        .map(|(_, text)| *text)
        .expect("a conformance model of additivity")
}

/// The model with the semi-additive metric of ADD-8, `debt_balance`, and the additive metric
/// `strict_balance` declared non-additive.
pub(crate) const MODEL: &str = MODELS[0].1;

/// The declaration of `debt_balance` in [`MODEL`], the example of ADD-8.
pub(crate) const EXAMPLE: &str = r#"{"type": "semi_additive", "over": "debt.period", "entity": ["debt.customer_id"], "aggregation": "last", "selection": "global_period", "snapshot_grain": "month", "population": "active_customers", "missing": "carry_forward", "max_carry_forward": 2, "on_stale": "error", "unexpected": "exclude", "nulls": "as_missing"}"#;

/// The JSON Pointer of `debt_balance` in [`MODEL`].
pub(crate) const DEBT_BALANCE: &str = "/semantic_model/0/metrics/0";

/// The JSON Pointer of `strict_balance` in [`MODEL`].
pub(crate) const STRICT_BALANCE: &str = "/semantic_model/0/metrics/1";

/// The JSON Pointer of the `data` member of the entry of `debt_balance` in [`MODEL`].
pub(crate) const DEBT_BALANCE_DATA: &str = "/semantic_model/0/metrics/0/custom_extensions/0/data";

/// [`MODEL`] with `debt_balance` declaring the JSON value `additivity`.
pub(crate) fn with_additivity(additivity: &str) -> String {
    let from = format!(r#""additivity": {EXAMPLE}}}'"#);
    assert!(MODEL.contains(&from), "the model has the example");
    MODEL.replacen(
        &from,
        &format!(r#""additivity": {}}}'"#, additivity.replace('\'', "''")),
        1,
    )
}

/// [`MODEL`] with `debt_balance` declaring [`EXAMPLE`] with `replacements`, each a pair of a
/// JSON member as written in the example and the text that replaces it.
pub(crate) fn with_example(replacements: &[(&str, &str)]) -> String {
    let mut additivity = EXAMPLE.to_owned();
    for (from, to) in replacements {
        assert!(additivity.contains(from), "the example has {from}");
        additivity = additivity.replacen(from, to, 1);
    }
    with_additivity(&additivity)
}

/// [`MODEL`] with the expression of `debt_balance`, its first metric, replaced by
/// `expression`.
pub(crate) fn with_debt_balance_expression(expression: &str) -> String {
    let from = "expression: 'SUM(debt.balance)'";
    assert!(MODEL.contains(from), "the model has debt_balance");
    let to = format!("expression: '{}'", expression.replace('\'', "''"));
    MODEL.replacen(from, &to, 1)
}
