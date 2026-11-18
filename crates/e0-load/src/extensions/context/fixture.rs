//! The `e0-context` conformance models and their variants, shared by the tests of the checks of
//! context metrics in E0.c, E0.d, E0.e, E0.f and E0.g.

/// The model document of every `e0-context*` conformance model, by name.
const MODELS: [(&str, &str); 15] = [
    (
        "e0-context",
        include_str!("../../../../../conformance/models/e0-context/model.yaml"),
    ),
    (
        "e0-context-wrong-value-001",
        include_str!("../../../../../conformance/models/e0-context-wrong-value-001/model.yaml"),
    ),
    (
        "e0-context-unknown-key-001",
        include_str!("../../../../../conformance/models/e0-context-unknown-key-001/model.yaml"),
    ),
    (
        "e0-context-additivity-001",
        include_str!("../../../../../conformance/models/e0-context-additivity-001/model.yaml"),
    ),
    (
        "e0-context-offset-001",
        include_str!("../../../../../conformance/models/e0-context-offset-001/model.yaml"),
    ),
    (
        "e0-context-base-001",
        include_str!("../../../../../conformance/models/e0-context-base-001/model.yaml"),
    ),
    (
        "e0-context-derived-001",
        include_str!("../../../../../conformance/models/e0-context-derived-001/model.yaml"),
    ),
    (
        "e0-context-nested-001",
        include_str!("../../../../../conformance/models/e0-context-nested-001/model.yaml"),
    ),
    (
        "e0-context-removed-field-001",
        include_str!("../../../../../conformance/models/e0-context-removed-field-001/model.yaml"),
    ),
    (
        "e0-context-unreachable-001",
        include_str!("../../../../../conformance/models/e0-context-unreachable-001/model.yaml"),
    ),
    (
        "e0-context-time-kind-001",
        include_str!("../../../../../conformance/models/e0-context-time-kind-001/model.yaml"),
    ),
    (
        "e0-context-cycle-001",
        include_str!("../../../../../conformance/models/e0-context-cycle-001/model.yaml"),
    ),
    (
        "e0-context-unusable",
        include_str!("../../../../../conformance/models/e0-context-unusable/model.yaml"),
    ),
    (
        "e0-context-development",
        include_str!("../../../../../conformance/models/e0-context-development/model.yaml"),
    ),
    (
        "e0-context-development-dataset-metric-001",
        include_str!(
            "../../../../../conformance/models/e0-context-development-dataset-metric-001/model.yaml"
        ),
    ),
];

/// The catalog schema of every `e0-context*` model, in both formats: `shop.main.invoices`
/// (`invoice_id`, `customer_id`, `invoice_date` date, `billed`) and `shop.main.customers`
/// (`customer_id`, `region` text); every integer is 32-bit and no column is nullable.
pub(crate) const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-context/catalog-schema.json");

/// The model document of the conformance model `name`.
pub(crate) fn conformance_model(name: &str) -> &'static str {
    MODELS
        .iter()
        .find(|(model, _)| *model == name)
        .map(|(_, text)| *text)
        .expect("a conformance model of context metrics")
}

/// The model with the three examples of CTX-4 over the base metric `billed`, the derived
/// metric `billed_change` that reads one of them (CTX-3.7), and the base metric
/// `customer_count`.
pub(crate) const MODEL: &str = MODELS[0].1;

/// The JSON Pointer of the base metric `billed` in [`MODEL`].
pub(crate) const BILLED: &str = "/semantic_model/0/metrics/0";

/// The JSON Pointer of `billed_ytd`, the `period_to_date` example of CTX-4, in [`MODEL`].
pub(crate) const BILLED_YTD: &str = "/semantic_model/0/metrics/1";

/// The JSON Pointer of `billed_prior_year`, the `prior_period` example of CTX-4, in [`MODEL`].
pub(crate) const BILLED_PRIOR_YEAR: &str = "/semantic_model/0/metrics/2";

/// The JSON Pointer of `billed_share`, the `share_of_total` example of CTX-4, in [`MODEL`].
pub(crate) const BILLED_SHARE: &str = "/semantic_model/0/metrics/3";

/// The JSON Pointer of the derived metric `billed_change` in [`MODEL`].
pub(crate) const BILLED_CHANGE: &str = "/semantic_model/0/metrics/4";

/// The JSON Pointer of the `data` member of the only entry of the release-format metric at
/// `metric`.
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

/// [`MODEL`] with the `context` of `billed_share`, the only one with `share_of_total`,
/// replaced by the JSON value `context`.
pub(crate) fn with_share_context(context: &str) -> String {
    with(
        r#""context": {"base": "billed", "modifier": {"type": "share_of_total", "remove": ["customers.region"]}}"#,
        &format!(r#""context": {context}"#),
    )
}
