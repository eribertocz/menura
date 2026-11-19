//! The `e0-security` conformance models and their variants, shared by the tests of the checks
//! of the security extension in E0.c, E0.d and E0.f.

/// The model document of every `e0-security*` conformance model, by name.
const MODELS: [(&str, &str); 18] = [
    (
        "e0-security",
        include_str!("../../../../../conformance/models/e0-security/model.yaml"),
    ),
    (
        "e0-security-aggregate-condition-001",
        include_str!(
            "../../../../../conformance/models/e0-security-aggregate-condition-001/model.yaml"
        ),
    ),
    (
        "e0-security-attribute-type-001",
        include_str!("../../../../../conformance/models/e0-security-attribute-type-001/model.yaml"),
    ),
    (
        "e0-security-development",
        include_str!("../../../../../conformance/models/e0-security-development/model.yaml"),
    ),
    (
        "e0-security-development-mode-001",
        include_str!(
            "../../../../../conformance/models/e0-security-development-mode-001/model.yaml"
        ),
    ),
    (
        "e0-security-duplicate-policy-001",
        include_str!(
            "../../../../../conformance/models/e0-security-duplicate-policy-001/model.yaml"
        ),
    ),
    (
        "e0-security-empty-roles-001",
        include_str!("../../../../../conformance/models/e0-security-empty-roles-001/model.yaml"),
    ),
    (
        "e0-security-field-role-001",
        include_str!("../../../../../conformance/models/e0-security-field-role-001/model.yaml"),
    ),
    (
        "e0-security-fields-001",
        include_str!("../../../../../conformance/models/e0-security-fields-001/model.yaml"),
    ),
    (
        "e0-security-internal-001",
        include_str!("../../../../../conformance/models/e0-security-internal-001/model.yaml"),
    ),
    (
        "e0-security-list-attribute-001",
        include_str!("../../../../../conformance/models/e0-security-list-attribute-001/model.yaml"),
    ),
    (
        "e0-security-mode-001",
        include_str!("../../../../../conformance/models/e0-security-mode-001/model.yaml"),
    ),
    (
        "e0-security-mode-keys-001",
        include_str!("../../../../../conformance/models/e0-security-mode-keys-001/model.yaml"),
    ),
    (
        "e0-security-no-context-001",
        include_str!("../../../../../conformance/models/e0-security-no-context-001/model.yaml"),
    ),
    (
        "e0-security-not-boolean-condition-001",
        include_str!(
            "../../../../../conformance/models/e0-security-not-boolean-condition-001/model.yaml"
        ),
    ),
    (
        "e0-security-undeclared-attribute-001",
        include_str!(
            "../../../../../conformance/models/e0-security-undeclared-attribute-001/model.yaml"
        ),
    ),
    (
        "e0-security-undeclared-role-001",
        include_str!(
            "../../../../../conformance/models/e0-security-undeclared-role-001/model.yaml"
        ),
    ),
    (
        "e0-security-unknown-key-001",
        include_str!("../../../../../conformance/models/e0-security-unknown-key-001/model.yaml"),
    ),
];

/// The catalog schema of every `e0-security*` model, in both formats: `shop.main.orders`
/// (`order_id`, `region` text, `amount`, `unit_cost`) and `shop.main.notes` (`note_id`,
/// `body` text); every integer is 32-bit and no column is nullable.
pub(crate) const CATALOG: &str =
    include_str!("../../../../../conformance/models/e0-security/catalog-schema.json");

/// The model document of the conformance model `name`.
pub(crate) fn conformance_model(name: &str) -> &'static str {
    MODELS
        .iter()
        .find(|(model, _)| *model == name)
        .map(|(_, text)| *text)
        .expect("a conformance model of the security extension")
}

/// The valid release-format model: `security_context` with the roles `admin` and `analyst`
/// and the attributes `regions` (`list<text>`), `area_id` (`integer`) and `limit`
/// (`decimal(18,2)`); `orders` with `access`, a field restriction on `unit_cost` and two row
/// policies; `notes` without `access`.
pub(crate) const MODEL: &str = MODELS[0].1;

/// The JSON Pointer of the semantic model of [`MODEL`].
pub(crate) const SEMANTIC_MODEL: &str = "/semantic_model/0";

/// The JSON Pointer of the dataset `orders` in [`MODEL`].
pub(crate) const ORDERS: &str = "/semantic_model/0/datasets/0";

/// The JSON Pointer of the dataset `notes` in [`MODEL`].
pub(crate) const NOTES: &str = "/semantic_model/0/datasets/1";

/// The JSON Pointer of the `data` member of the entry of the semantic model of [`MODEL`].
pub(crate) const MODEL_DATA: &str = "/semantic_model/0/custom_extensions/0/data";

/// The JSON Pointer of the `data` member of the entry of `orders` in [`MODEL`].
pub(crate) const ORDERS_DATA: &str = "/semantic_model/0/datasets/0/custom_extensions/0/data";

/// The JSON Pointer of the `data` member of the entry of `notes`, in the models that give it
/// one.
pub(crate) const NOTES_DATA: &str = "/semantic_model/0/datasets/1/custom_extensions/0/data";

/// The text of the `row_access` of `orders` in [`MODEL`], as it is written in the YAML text.
pub(crate) const ROW_ACCESS: &str = r#""row_access": [{"name": "region_scope", "condition": "region IN (USER_ATTRIBUTE(''regions''))", "roles": ["analyst"]}, {"name": "open_scope", "condition": "TRUE", "roles": ["admin"]}]"#;

/// The text of the `access` of `orders` in [`MODEL`].
pub(crate) const ACCESS: &str = r#""access": {"mode": "roles", "roles": ["analyst", "admin"], "fields": [{"names": ["unit_cost"], "roles": ["admin"]}]}"#;

/// The text of the `security_context` of [`MODEL`].
pub(crate) const SECURITY_CONTEXT: &str = r#""security_context": {"roles": ["admin", "analyst"], "attributes": [{"name": "regions", "type": "list<text>"}, {"name": "area_id", "type": "integer"}, {"name": "limit", "type": "decimal(18,2)"}]}"#;

/// [`MODEL`] with the text `from`, written once in it, replaced by `to`.
pub(crate) fn with(from: &str, to: &str) -> String {
    assert_eq!(
        MODEL.matches(from).count(),
        1,
        "the model has `{from}` once"
    );
    MODEL.replacen(from, to, 1)
}

/// [`MODEL`] with the `row_access` of `orders` replaced by `row_access`, a JSON value whose
/// single quotes are doubled as the YAML text requires.
pub(crate) fn with_row_access(row_access: &str) -> String {
    with(ROW_ACCESS, &format!(r#""row_access": {row_access}"#))
}

/// [`MODEL`] with the `access` of `orders` replaced by the JSON value `access`.
pub(crate) fn with_access(access: &str) -> String {
    with(ACCESS, &format!(r#""access": {access}"#))
}

/// [`MODEL`] with the `security_context` replaced by the JSON value `context`.
pub(crate) fn with_security_context(context: &str) -> String {
    with(
        SECURITY_CONTEXT,
        &format!(r#""security_context": {context}"#),
    )
}

/// [`MODEL`] where `notes` has an entry of ours with the members `members`.
pub(crate) fn with_notes(members: &str) -> String {
    with(
        "            expression: {dialects: [{dialect: ANSI_SQL, expression: 'body'}]}\n",
        &format!(
            "            expression: {{dialects: [{{dialect: ANSI_SQL, expression: 'body'}}]}}\n        custom_extensions:\n          - vendor_name: COMMON\n            data: '{{\"namespace\": \"semantic_compiler\", \"version\": 1, {members}}}'\n"
        ),
    )
}
