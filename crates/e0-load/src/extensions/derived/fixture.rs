//! The `e0-derived` conformance models and their variants, shared by the tests of the checks of
//! derived metrics in E0.c, E0.d, E0.f and E0.g.

/// The model document of every `e0-derived*` conformance model, by name.
const MODELS: [(&str, &str); 12] = [
    (
        "e0-derived",
        include_str!("../../../../../conformance/models/e0-derived/model.yaml"),
    ),
    (
        "e0-derived-wrong-value-001",
        include_str!("../../../../../conformance/models/e0-derived-wrong-value-001/model.yaml"),
    ),
    (
        "e0-derived-unknown-key-001",
        include_str!("../../../../../conformance/models/e0-derived-unknown-key-001/model.yaml"),
    ),
    (
        "e0-derived-additivity-001",
        include_str!("../../../../../conformance/models/e0-derived-additivity-001/model.yaml"),
    ),
    (
        "e0-derived-unknown-metric-001",
        include_str!("../../../../../conformance/models/e0-derived-unknown-metric-001/model.yaml"),
    ),
    (
        "e0-derived-no-metric-001",
        include_str!("../../../../../conformance/models/e0-derived-no-metric-001/model.yaml"),
    ),
    (
        "e0-derived-aggregate-001",
        include_str!("../../../../../conformance/models/e0-derived-aggregate-001/model.yaml"),
    ),
    (
        "e0-derived-field-reference-001",
        include_str!("../../../../../conformance/models/e0-derived-field-reference-001/model.yaml"),
    ),
    (
        "e0-derived-cycle-001",
        include_str!("../../../../../conformance/models/e0-derived-cycle-001/model.yaml"),
    ),
    (
        "e0-derived-division-001",
        include_str!("../../../../../conformance/models/e0-derived-division-001/model.yaml"),
    ),
    (
        "e0-derived-unusable",
        include_str!("../../../../../conformance/models/e0-derived-unusable/model.yaml"),
    ),
    (
        "e0-derived-development",
        include_str!("../../../../../conformance/models/e0-derived-development/model.yaml"),
    ),
];

/// The catalog schema of every release-format `e0-derived*` model: `shop.invoices`
/// (`invoice_id`, `customer_id`, `billed`, `collected`) and `shop.customers`
/// (`customer_id`, `region` text); every integer is 32-bit and no column is nullable.
pub(crate) const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-derived/catalog-schema.json");

/// The catalog schema of the `e0-derived-development` model: `shop.orders` (`order_id`,
/// `customer_id`, `status` text, `amount`) and `shop.customers`.
pub(crate) const DEVELOPMENT_CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-derived-development/catalog-schema.json");

/// The model document of the conformance model `name`.
pub(crate) fn conformance_model(name: &str) -> &'static str {
    MODELS
        .iter()
        .find(|(model, _)| *model == name)
        .map(|(_, text)| *text)
        .expect("a conformance model of derived metrics")
}

/// The model with the example of DER-2, `collection_rate`, the derived metric
/// `uncollected_rate` that reads it, and `billed_per_customer`, whose inputs have different
/// home datasets.
pub(crate) const MODEL: &str = MODELS[0].1;

/// The JSON Pointer of the base metric `billed` in [`MODEL`].
pub(crate) const BILLED: &str = "/semantic_model/0/metrics/0";

/// The JSON Pointer of the base metric `collected` in [`MODEL`].
pub(crate) const COLLECTED: &str = "/semantic_model/0/metrics/1";

/// The JSON Pointer of `collection_rate` in [`MODEL`], the example of DER-2.
pub(crate) const COLLECTION_RATE: &str = "/semantic_model/0/metrics/2";

/// The JSON Pointer of `uncollected_rate` in [`MODEL`].
pub(crate) const UNCOLLECTED_RATE: &str = "/semantic_model/0/metrics/3";

/// The JSON Pointer of the base metric `customer_count` in [`MODEL`].
pub(crate) const CUSTOMER_COUNT: &str = "/semantic_model/0/metrics/4";

/// The JSON Pointer of `billed_per_customer` in [`MODEL`].
pub(crate) const BILLED_PER_CUSTOMER: &str = "/semantic_model/0/metrics/5";

/// The JSON Pointer of the `data` member of the only entry of the metric at `metric`.
pub(crate) fn data(metric: &str) -> String {
    format!("{metric}/custom_extensions/0/data")
}

/// [`MODEL`] with the text `from`, written once in it, replaced by `to`.
pub(crate) fn with(from: &str, to: &str) -> String {
    assert_eq!(
        MODEL.matches(from).count(),
        1,
        "the model has `{from}` once"
    );
    MODEL.replacen(from, to, 1)
}

/// [`MODEL`] with the formula of `uncollected_rate`, `1 - AGG(collection_rate)`, replaced by
/// `formula`.
pub(crate) fn with_uncollected_rate(formula: &str) -> String {
    with(
        "'1 - AGG(collection_rate)'",
        &format!("'{}'", formula.replace('\'', "''")),
    )
}

/// [`MODEL`] with the `derived` of `uncollected_rate`, `{}`, replaced by the JSON value
/// `derived`.
pub(crate) fn with_uncollected_derived(derived: &str) -> String {
    with(r#""derived": {}"#, &format!(r#""derived": {derived}"#))
}
