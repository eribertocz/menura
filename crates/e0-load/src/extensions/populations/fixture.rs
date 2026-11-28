//! The `e0-populations` conformance model and its variants, shared by the tests of the checks
//! of populations in E0.c, E0.d and E0.f.

/// The model document of every `e0-populations*` conformance model, by name.
const MODELS: [(&str, &str); 11] = [
    (
        "e0-populations",
        include_str!("../../../../../conformance/models/e0-populations/model.yaml"),
    ),
    (
        "e0-populations-duplicate-name-001",
        include_str!(
            "../../../../../conformance/models/e0-populations-duplicate-name-001/model.yaml"
        ),
    ),
    (
        "e0-populations-mode-keys-001",
        include_str!("../../../../../conformance/models/e0-populations-mode-keys-001/model.yaml"),
    ),
    (
        "e0-populations-unknown-key-001",
        include_str!("../../../../../conformance/models/e0-populations-unknown-key-001/model.yaml"),
    ),
    (
        "e0-populations-wrong-value-001",
        include_str!("../../../../../conformance/models/e0-populations-wrong-value-001/model.yaml"),
    ),
    (
        "e0-populations-missing-field-001",
        include_str!(
            "../../../../../conformance/models/e0-populations-missing-field-001/model.yaml"
        ),
    ),
    (
        "e0-populations-invalid-condition-001",
        include_str!(
            "../../../../../conformance/models/e0-populations-invalid-condition-001/model.yaml"
        ),
    ),
    (
        "e0-populations-period-kind-001",
        include_str!("../../../../../conformance/models/e0-populations-period-kind-001/model.yaml"),
    ),
    (
        "e0-populations-untyped-field-condition-001",
        include_str!(
            "../../../../../conformance/models/e0-populations-untyped-field-condition-001/model.yaml"
        ),
    ),
    (
        "val-1-5-open-end-value-001",
        include_str!("../../../../../conformance/models/val-1-5-open-end-value-001/model.yaml"),
    ),
    (
        "val-1-5-open-end-untyped-001",
        include_str!("../../../../../conformance/models/val-1-5-open-end-untyped-001/model.yaml"),
    ),
];

/// The model document of the conformance model `name`.
pub(crate) fn conformance_model(name: &str) -> &'static str {
    MODELS
        .iter()
        .find(|(model, _)| *model == name)
        .map(|(_, text)| *text)
        .expect("a conformance model of populations")
}

/// The model with a valid validity population and a valid periodic population.
pub(crate) const MODEL: &str = MODELS[0].1;

/// The population of the dataset `customers` in [`MODEL`].
pub(crate) const VALIDITY: &str = r#"{"name": "active_customers", "mode": "validity", "entity": ["customer_id"], "valid_from": "valid_from", "valid_to": "valid_to", "partial_period": "any_day", "required_when": "active"}"#;

/// The population of the dataset `account_months` in [`MODEL`].
pub(crate) const PERIODIC: &str = r#"{"name": "monthly_accounts", "mode": "periodic", "entity": ["account_id"], "period": "period_start", "grain": "month"}"#;

pub(crate) const CUSTOMERS: &str = "/semantic_model/0/datasets/0";
pub(crate) const CUSTOMERS_DATA: &str = "/semantic_model/0/datasets/0/custom_extensions/0/data";
pub(crate) const ACCOUNTS_DATA: &str = "/semantic_model/0/datasets/1/custom_extensions/0/data";

/// [`MODEL`] with `customers` and `account_months` carrying the JSON `populations` values
/// `validity` and `periodic`.
pub(crate) fn with_populations(validity: &str, periodic: &str) -> String {
    let replace = |model: &str, from: &str, to: &str| {
        let from = format!(r#""populations": [{from}]"#);
        assert!(model.contains(&from), "the model has the population");
        model.replacen(
            &from,
            &format!(r#""populations": {}"#, to.replace('\'', "''")),
            1,
        )
    };
    replace(&replace(MODEL, VALIDITY, validity), PERIODIC, periodic)
}

/// [`MODEL`] with the item `validity` on `customers`.
pub(crate) fn with_validity(validity: &str) -> String {
    with_populations(&format!("[{validity}]"), &format!("[{PERIODIC}]"))
}
