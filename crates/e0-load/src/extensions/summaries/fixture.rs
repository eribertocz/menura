//! The `e0-summar*` conformance models and their variants, shared by the tests of the checks
//! of the summary extension in E0.c, E0.d and E0.f.

/// The model document of every `e0-summar*` conformance model, by name.
const MODELS: [(&str, &str); 14] = [
    (
        "e0-summaries",
        include_str!("../../../../../conformance/models/e0-summaries/model.yaml"),
    ),
    (
        "e0-summaries-development",
        include_str!("../../../../../conformance/models/e0-summaries-development/model.yaml"),
    ),
    (
        "e0-summary-access-001",
        include_str!("../../../../../conformance/models/e0-summary-access-001/model.yaml"),
    ),
    (
        "e0-summary-base-aggregate-001",
        include_str!("../../../../../conformance/models/e0-summary-base-aggregate-001/model.yaml"),
    ),
    (
        "e0-summary-coverage-001",
        include_str!("../../../../../conformance/models/e0-summary-coverage-001/model.yaml"),
    ),
    (
        "e0-summary-dataset-metrics-001",
        include_str!("../../../../../conformance/models/e0-summary-dataset-metrics-001/model.yaml"),
    ),
    (
        "e0-summary-grain-001",
        include_str!("../../../../../conformance/models/e0-summary-grain-001/model.yaml"),
    ),
    (
        "e0-summary-kind-001",
        include_str!("../../../../../conformance/models/e0-summary-kind-001/model.yaml"),
    ),
    (
        "e0-summary-metric-001",
        include_str!("../../../../../conformance/models/e0-summary-metric-001/model.yaml"),
    ),
    (
        "e0-summary-missing-field-001",
        include_str!("../../../../../conformance/models/e0-summary-missing-field-001/model.yaml"),
    ),
    (
        "e0-summary-missing-of-001",
        include_str!("../../../../../conformance/models/e0-summary-missing-of-001/model.yaml"),
    ),
    (
        "e0-summary-of-summary-001",
        include_str!("../../../../../conformance/models/e0-summary-of-summary-001/model.yaml"),
    ),
    (
        "e0-summary-row-access-001",
        include_str!("../../../../../conformance/models/e0-summary-row-access-001/model.yaml"),
    ),
    (
        "e0-summary-unknown-key-001",
        include_str!("../../../../../conformance/models/e0-summary-unknown-key-001/model.yaml"),
    ),
];

/// The valid model: `orders` (`order_id`, `region`, `order_date`, `amount`) and its summary
/// `orders_daily` (`order_date`, `region`, `total_amount`, `order_count`).
pub(crate) const MODEL: &str = MODELS[0].1;

/// The catalog schema of every model but `e0-summary-kind-001`: `shop.orders`
/// (`order_id` 32-bit, `region` text, `order_date` date, `amount` 32-bit) and
/// `shop.orders_daily` (`order_date` date, `region` text, `total_amount` and
/// `order_count` 64-bit).
pub(crate) const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-summaries/catalog-schema.json");

/// The catalog schema of `e0-summary-kind-001`, where `total_amount` is text.
pub(crate) const KIND_CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-summary-kind-001/catalog-schema.json");

/// The JSON Pointer of the summary dataset `orders_daily`.
pub(crate) const SUMMARY: &str = "/semantic_model/0/datasets/1";

/// The JSON Pointer of the `data` member of the entry of ours of `orders_daily`.
pub(crate) const SUMMARY_DATA: &str = "/semantic_model/0/datasets/1/custom_extensions/0/data";

/// The JSON Pointer of the base dataset `orders`.
pub(crate) const ORDERS: &str = "/semantic_model/0/datasets/0";

/// The `access` of the summary in [`MODEL`].
pub(crate) const ACCESS: &str = r#""access": {"mode": "internal"}"#;

/// The `aggregate` of the summary in [`MODEL`], as written in its JSON text.
pub(crate) const AGGREGATE: &str = r#"{"of": "orders", "dimensions": [{"field": "order_date", "base": "order_date", "grain": "day"}, {"field": "region", "base": "region"}], "measures": [{"field": "total_amount", "base_aggregate": "SUM(orders.amount)"}, {"field": "order_count", "base_aggregate": "COUNT(orders.order_id)"}], "coverage_field": "order_date"}"#;

/// The model document of the conformance model `name`.
pub(crate) fn conformance_model(name: &str) -> &'static str {
    MODELS
        .iter()
        .find(|(model, _)| *model == name)
        .map(|(_, text)| *text)
        .expect("the conformance model exists")
}

/// [`MODEL`] where the extension keys of the summary entry, `access` and `aggregate`, are
/// `members`, a JSON text.
pub(crate) fn with_members(members: &str) -> String {
    let written = format!(r#"{ACCESS}, "aggregate": {AGGREGATE}"#);
    assert!(MODEL.contains(&written), "the summary entry is found");
    MODEL.replacen(&written, &members.replace('\'', "''"), 1)
}

/// [`MODEL`] where the summary has the `aggregate` value `aggregate`, a JSON text.
pub(crate) fn with_aggregate(aggregate: &str) -> String {
    with_members(&format!(r#"{ACCESS}, "aggregate": {aggregate}"#))
}

/// [`MODEL`] where the text `from` of [`AGGREGATE`] is replaced by `to`.
pub(crate) fn with_changed(from: &str, to: &str) -> String {
    assert!(
        AGGREGATE.contains(from),
        "`{from}` is part of the aggregate"
    );
    with_aggregate(&AGGREGATE.replacen(from, to, 1))
}

/// [`MODEL`] where the text `from` of the whole document is replaced by `to`.
pub(crate) fn with_text(from: &str, to: &str) -> String {
    assert!(MODEL.contains(from), "`{from}` is part of the model");
    MODEL.replacen(from, to, 1)
}
