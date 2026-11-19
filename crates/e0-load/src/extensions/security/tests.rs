//! The security extension in E0.c: SCY-2.1, SCY-2.2, SCY-3, SCY-3.0, SCY-4, SCY-5,
//! SCY-5.1.1 to SCY-5.1.3, SCY-5.1.5, SCY-5.1.7, DEV-4, `E0C-02`, `E0C-03`, `E0C-41` to
//! `E0C-45`, `E0C-47`, `W0C-40` and `W0C-41`.
//!
//! The model documents of the `e0-security*` conformance cases are read here at compile time
//! (see [`super::fixture`]), so the rules are checked against the files the conformance suite
//! uses.

use semantic_compiler_security::{
    AccessMode, AttributeType, DatasetAccess, FieldRestriction, Role, ScalarType,
};

use super::fixture::{
    ACCESS, MODEL, MODEL_DATA, NOTES_DATA, ORDERS, ORDERS_DATA, ROW_ACCESS, SECURITY_CONTEXT,
    SEMANTIC_MODEL, conformance_model, with_notes, with_row_access, with_security_context,
};
use super::support::{codes_at, rejected, validate};

fn row_access_codes(row_access: &str) -> Vec<&'static str> {
    codes_at(&with_row_access(row_access), ORDERS_DATA)
}

fn context_codes(context: &str) -> Vec<&'static str> {
    codes_at(&with_security_context(context), MODEL_DATA)
}

fn roles(names: &[&str]) -> Vec<Role> {
    names.iter().map(|name| Role::new(name)).collect()
}

#[test]
fn scy_5_1_2_scy_5_1_3_scy_5_1_5_the_valid_model_is_read() {
    let security = validate(MODEL)
        .expect("the extensions are accepted")
        .security;
    assert_eq!(security.model.to_string(), SEMANTIC_MODEL);
    let context = security.context.expect("a security context");
    assert_eq!(context.roles, roles(&["admin", "analyst"]));
    assert_eq!(
        context.attribute("regions"),
        Some(AttributeType::List(ScalarType::Text))
    );
    assert_eq!(
        context.attribute("area_id"),
        Some(AttributeType::Scalar(ScalarType::Integer))
    );
    assert_eq!(
        context.attribute("limit"),
        Some(AttributeType::Scalar(ScalarType::Decimal {
            precision: 18,
            scale: 2
        }))
    );
    assert_eq!(context.attribute("Regions"), None);
    assert_eq!(security.accesses.len(), 1);
    assert_eq!(security.accesses[0].dataset.to_string(), ORDERS);
    assert_eq!(security.accesses[0].data.to_string(), ORDERS_DATA);
    assert_eq!(
        security.accesses[0].access,
        DatasetAccess {
            mode: AccessMode::Roles(roles(&["analyst", "admin"])),
            fields: vec![FieldRestriction {
                names: vec!["unit_cost".to_owned()],
                roles: roles(&["admin"]),
            }],
        }
    );
    let policies: Vec<(&str, &str)> = security
        .policies
        .iter()
        .map(|policy| (policy.name.as_str(), policy.condition.as_str()))
        .collect();
    assert_eq!(
        policies,
        [
            ("region_scope", "region IN (USER_ATTRIBUTE('regions'))"),
            ("open_scope", "TRUE")
        ]
    );
    assert_eq!(security.policies[0].roles, Some(roles(&["analyst"])));
    assert_eq!(security.policies[0].exempt_roles, None);
}

#[test]
fn scy_2_1_roles_and_attributes_may_be_empty() {
    let model = MODEL
        .replacen(
            SECURITY_CONTEXT,
            r#""security_context": {"roles": [], "attributes": []}"#,
            1,
        )
        .replacen(ACCESS, r#""access": {"mode": "authenticated"}"#, 1)
        .replacen(
            ROW_ACCESS,
            r#""row_access": [{"name": "open_scope", "condition": "TRUE"}]"#,
            1,
        );
    let security = validate(&model)
        .expect("the extensions are accepted")
        .security;
    let context = security.context.expect("a security context");
    assert!(context.roles.is_empty() && context.attributes.is_empty());
    assert_eq!(security.policies.len(), 1);
}

#[test]
fn scy_5_1_2_e0c_02_e0c_03_the_shape_of_security_context() {
    let attributes = r#""attributes": []"#;
    for (context, expected) in [
        (format!(r#"{{"roles": ["admin", "analyst"], {attributes}, "groups": []}}"#), vec!["E0C-02"]),
        (format!(r"{{{attributes}}}"), vec!["E0C-03"]),
        (r#"{"roles": ["admin", "analyst"]}"#.to_owned(), vec!["E0C-03"]),
        (r#"["admin"]"#.to_owned(), vec!["E0C-03"]),
        (format!(r#"{{"roles": "admin", {attributes}}}"#), vec!["E0C-03"]),
        (format!(r#"{{"roles": ["admin", "analyst", ""], {attributes}}}"#), vec!["E0C-03"]),
        (format!(r#"{{"roles": ["admin", "analyst", "admin"], {attributes}}}"#), vec!["E0C-03"]),
        (format!(r#"{{"roles": ["admin", "analyst", 7], {attributes}}}"#), vec!["E0C-03"]),
        (r#"{"roles": ["admin", "analyst"], "attributes": {}}"#.to_owned(), vec!["E0C-03"]),
        (r#"{"roles": ["admin", "analyst"], "attributes": [{"name": "regions"}]}"#.to_owned(), vec!["E0C-03"]),
        (r#"{"roles": ["admin", "analyst"], "attributes": [{"name": "", "type": "text"}]}"#.to_owned(), vec!["E0C-03"]),
        (r#"{"roles": ["admin", "analyst"], "attributes": [{"name": "a", "type": "string"}]}"#.to_owned(), vec!["E0C-03"]),
        (r#"{"roles": ["admin", "analyst"], "attributes": [{"name": "a", "type": "decimal(18)"}]}"#.to_owned(), vec!["E0C-03"]),
        (r#"{"roles": ["admin", "analyst"], "attributes": [{"name": "a", "type": "list<list<text>>"}]}"#.to_owned(), vec!["E0C-03"]),
        (r#"{"roles": ["admin", "analyst"], "attributes": [{"name": "a", "type": "text"}, {"name": "a", "type": "date"}]}"#.to_owned(), vec!["E0C-03"]),
        (r#"{"roles": ["admin", "analyst"], "attributes": [{"name": "a", "type": "text", "default": 1}]}"#.to_owned(), vec!["E0C-02"]),
        (r#"{"roles": ["admin", "analyst"], "attributes": ["a"]}"#.to_owned(), vec!["E0C-03"]),
    ] {
        assert_eq!(context_codes(&context), expected, "{context}");
    }
}

#[test]
fn scy_2_2_every_attribute_type_is_accepted() {
    for written in [
        "text",
        "integer",
        "decimal(1,0)",
        "decimal(38,38)",
        "date",
        "boolean",
        "list<date>",
        "list<decimal(5,2)>",
    ] {
        let context = format!(
            r#"{{"roles": ["admin", "analyst"], "attributes": [{{"name": "a", "type": "{written}"}}]}}"#
        );
        assert!(
            validate(&with_security_context(&context)).is_ok(),
            "{written}"
        );
    }
}

#[test]
fn scy_2_1_e0c_47_without_security_context_no_role_is_declared() {
    let lines: Vec<&str> = MODEL.lines().collect();
    assert!(lines[4].ends_with("custom_extensions:") && lines[6].contains("security_context"));
    let without_context: Vec<&str> = lines
        .iter()
        .enumerate()
        .filter(|(index, _)| !(4..=6).contains(index))
        .map(|(_, line)| *line)
        .collect();
    let model = without_context.join("\n") + "\n";
    let codes: Vec<(&str, String)> = rejected(&model);
    assert_eq!(codes, [("E0C-47", ORDERS_DATA.to_owned())]);
}

#[test]
fn scy_5_1_5_e0c_02_e0c_03_the_shape_of_row_access() {
    for row_access in [
        r#"{"name": "a", "condition": "TRUE"}"#,
        r#"["a"]"#,
        r#"[{"condition": "TRUE"}]"#,
        r#"[{"name": "a"}]"#,
        r#"[{"name": 1, "condition": "TRUE"}]"#,
        r#"[{"name": "a", "condition": true}]"#,
        r#"[{"name": "a", "condition": "TRUE", "roles": []}]"#,
        r#"[{"name": "a", "condition": "TRUE", "roles": ["admin", "admin"]}]"#,
        r#"[{"name": "a", "condition": "TRUE", "exempt_roles": "admin"}]"#,
    ] {
        assert_eq!(row_access_codes(row_access), ["E0C-03"], "{row_access}");
    }
    assert_eq!(
        row_access_codes(r#"[{"name": "a", "condition": "TRUE", "mask": "x"}]"#),
        ["E0C-02"]
    );
    let valid =
        r#"[{"name": "a", "condition": "TRUE", "roles": ["admin"], "exempt_roles": ["analyst"]}]"#;
    let security = validate(&with_row_access(valid))
        .expect("the extensions are accepted")
        .security;
    assert_eq!(security.policies[0].exempt_roles, Some(roles(&["analyst"])));
}

#[test]
fn scy_5_1_1_scy_5_1_5_e0c_45_policy_names_are_unique_in_the_model() {
    let twice = r#"[{"name": "a", "condition": "TRUE"}, {"name": "a", "condition": "FALSE"}]"#;
    assert_eq!(row_access_codes(twice), ["E0C-45"]);
    let other_dataset = with_notes(
        r#""access": {"mode": "authenticated"}, "row_access": [{"name": "REGION_SCOPE", "condition": "TRUE"}]"#,
    );
    assert_eq!(codes_at(&other_dataset, NOTES_DATA), ["E0C-45"]);
    let delimited = |name: &str| {
        with_notes(&format!(
            r#""access": {{"mode": "authenticated"}}, "row_access": [{{"name": "\"{name}\"", "condition": "TRUE"}}]"#
        ))
    };
    assert_eq!(codes_at(&delimited("REGION_SCOPE"), NOTES_DATA), ["E0C-45"]);
    assert!(validate(&delimited("region_scope")).is_ok());
}

#[test]
fn scy_5_1_5_e0c_47_undeclared_roles_of_row_access() {
    let roles = r#"[{"name": "a", "condition": "TRUE", "roles": ["auditor"]}]"#;
    assert_eq!(row_access_codes(roles), ["E0C-47"]);
    let exempt = r#"[{"name": "a", "condition": "TRUE", "exempt_roles": ["auditor"]}]"#;
    assert_eq!(row_access_codes(exempt), ["E0C-47"]);
}

#[test]
fn scy_5_1_2_an_unreadable_role_list_reports_no_undeclared_role() {
    let model = with_security_context(r#"{"roles": "admin", "attributes": []}"#);
    let model = model.replace(r#""roles": ["analyst"]"#, r#""roles": ["auditor"]"#);
    assert_eq!(rejected(&model), [("E0C-03", MODEL_DATA.to_owned())]);
}

#[test]
fn scy_5_the_e0c_conformance_models_are_rejected_with_their_code() {
    for (name, code, data) in [
        ("e0-security-unknown-key-001", "E0C-02", MODEL_DATA),
        ("e0-security-attribute-type-001", "E0C-03", MODEL_DATA),
        ("e0-security-mode-001", "E0C-41", ORDERS_DATA),
        ("e0-security-empty-roles-001", "E0C-42", ORDERS_DATA),
        ("e0-security-repeated-role-001", "E0C-03", ORDERS_DATA),
        ("e0-security-mode-keys-001", "E0C-43", ORDERS_DATA),
        ("e0-security-internal-001", "E0C-44", NOTES_DATA),
        ("e0-security-duplicate-policy-001", "E0C-45", NOTES_DATA),
        ("e0-security-undeclared-role-001", "E0C-47", ORDERS_DATA),
    ] {
        assert_eq!(
            rejected(conformance_model(name)),
            [(code, data.to_owned())],
            "{name}"
        );
    }
    for name in [
        "e0-security-no-context-001",
        "e0-security-undeclared-attribute-001",
        "e0-security-aggregate-condition-001",
        "e0-security-not-boolean-condition-001",
        "e0-security-list-attribute-001",
        "e0-security-fields-001",
        "e0-security-field-role-001",
    ] {
        assert!(validate(conformance_model(name)).is_ok(), "{name}");
    }
}
