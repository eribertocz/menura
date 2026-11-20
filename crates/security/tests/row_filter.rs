//! Row-policy evaluation for one dataset and one user (E4-6.2 to E4-6.4, E4-6.9, E4-6.9a,
//! SEC-3.4, SEC-3.5).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::compiled_plan::InputDocument;
use menura_model::expression::{BinaryOperator, Expression, Identifier, Literal, Reference};
use menura_model::internal_model::{
    Dataset, Descriptions, NormalizedName, RoleName, RowPolicy, ScalarType, SecurityContext,
    SourceLocation,
};
use menura_security::{
    AdmittedSecurityContext, RowFilterError, admit_security_context, row_filter,
};
use support::{declarations, document, list, scalar};

fn location() -> SourceLocation {
    SourceLocation {
        pointer: "/semantic_model/0/datasets/0/custom_extensions/0/data".to_owned(),
        line: 1,
        column: 1,
    }
}

fn roles(names: &[&str]) -> Vec<RoleName> {
    names.iter().map(|name| RoleName::new(name)).collect()
}

fn policy(
    name: &str,
    condition: Expression,
    granted: Option<Vec<RoleName>>,
    exempt: Option<Vec<RoleName>>,
) -> RowPolicy {
    RowPolicy {
        name: name.to_owned(),
        condition: Some(condition),
        roles: granted,
        exempt_roles: exempt,
        location: location(),
    }
}

fn dataset(policies: Vec<RowPolicy>) -> Dataset {
    Dataset {
        name: NormalizedName::new("ORDERS"),
        source: "main.orders".to_owned(),
        fields: Vec::new(),
        metrics: Vec::new(),
        primary_key: Vec::new(),
        unique_keys: Vec::new(),
        access: None,
        row_policies: policies,
        populations: Vec::new(),
        summary: false,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

fn field(name: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::regular(name).expect("a regular identifier"),
    })
}

fn attribute(name: &str) -> Expression {
    Expression::UserAttribute(name.to_owned())
}

fn equals(left: Expression, right: Expression) -> Expression {
    binary(BinaryOperator::Equal, left, right)
}

fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn and(left: Expression, right: Expression) -> Expression {
    binary(BinaryOperator::And, left, right)
}

fn or(left: Expression, right: Expression) -> Expression {
    binary(BinaryOperator::Or, left, right)
}

fn boolean(value: bool) -> Expression {
    Expression::Literal(Literal::Boolean(value))
}

fn in_list(operand: Expression, item: Expression, negated: bool) -> Expression {
    Expression::InList {
        operand: Box::new(operand),
        list: vec![item],
        negated,
    }
}

fn model_declarations() -> SecurityContext {
    declarations(
        &["admin", "analyst", "auditor", "manager"],
        &[
            ("region", scalar(ScalarType::Text)),
            ("regions", list(ScalarType::Text)),
            ("team", scalar(ScalarType::Text)),
        ],
    )
}

fn user(roles: &str, attributes: &str) -> AdmittedSecurityContext {
    admit_security_context(&document(roles, attributes), Some(&model_declarations()))
        .expect("an admissible security context")
}

fn region_policy(granted: Option<Vec<RoleName>>, exempt: Option<Vec<RoleName>>) -> RowPolicy {
    policy(
        "region_scope",
        equals(field("region"), attribute("region")),
        granted,
        exempt,
    )
}

fn team_policy(granted: Option<Vec<RoleName>>) -> RowPolicy {
    policy(
        "team_scope",
        equals(field("team"), attribute("team")),
        granted,
        None,
    )
}

#[test]
fn e4_6_2_a_dataset_without_policies_has_no_row_filter() {
    let filter = row_filter(&dataset(Vec::new()), &user("[]", "{}"));
    assert_eq!(filter, Ok(None));
}

#[test]
fn e4_6_2_an_unscoped_policy_restricts_everyone_as_r() {
    let filter = row_filter(
        &dataset(vec![region_policy(None, None)]),
        &user(r#"["analyst"]"#, r#"{"region": "EU"}"#),
    );
    let condition = equals(field("region"), attribute("region"));
    assert_eq!(filter, Ok(Some(and(boolean(true), condition))));
}

#[test]
fn e4_6_2_unscoped_policies_are_combined_with_and() {
    let filter = row_filter(
        &dataset(vec![region_policy(None, None), team_policy(None)]),
        &user("[]", r#"{"region": "EU", "team": "north"}"#),
    );
    let region = equals(field("region"), attribute("region"));
    let team = equals(field("team"), attribute("team"));
    assert_eq!(filter, Ok(Some(and(boolean(true), and(region, team)))));
}

#[test]
fn e4_6_2_granting_role_scoped_policies_are_combined_with_or() {
    let filter = row_filter(
        &dataset(vec![
            region_policy(Some(roles(&["analyst"])), None),
            team_policy(Some(roles(&["manager"]))),
            policy("admin_scope", boolean(true), Some(roles(&["admin"])), None),
        ]),
        &user(
            r#"["analyst", "manager"]"#,
            r#"{"region": "EU", "team": "north"}"#,
        ),
    );
    let region = equals(field("region"), attribute("region"));
    let team = equals(field("team"), attribute("team"));
    assert_eq!(filter, Ok(Some(and(or(region, team), boolean(true)))));
}

#[test]
fn e4_6_2_only_the_granting_policy_of_two_role_scoped_policies_is_kept() {
    let filter = row_filter(
        &dataset(vec![
            region_policy(Some(roles(&["analyst"])), None),
            team_policy(Some(roles(&["manager"]))),
        ]),
        &user(r#"["analyst"]"#, r#"{"region": "EU"}"#),
    );
    let region = equals(field("region"), attribute("region"));
    assert_eq!(filter, Ok(Some(and(region, boolean(true)))));
}

#[test]
fn e4_6_3_an_exemption_removes_only_that_policy() {
    let filter = row_filter(
        &dataset(vec![
            region_policy(None, Some(roles(&["auditor"]))),
            team_policy(None),
        ]),
        &user(r#"["auditor"]"#, r#"{"team": "north"}"#),
    );
    let team = equals(field("team"), attribute("team"));
    assert_eq!(filter, Ok(Some(and(boolean(true), team))));
}

#[test]
fn e4_6_3_an_exemption_from_every_role_scoped_policy_makes_g_true() {
    let filter = row_filter(
        &dataset(vec![
            region_policy(Some(roles(&["analyst"])), Some(roles(&["auditor"]))),
            policy(
                "team_scope",
                equals(field("team"), attribute("team")),
                Some(roles(&["manager"])),
                Some(roles(&["auditor"])),
            ),
        ]),
        &user(r#"["auditor"]"#, "{}"),
    );
    assert_eq!(filter, Ok(Some(and(boolean(true), boolean(true)))));
}

#[test]
fn e4_6_4_no_granting_policy_is_the_constant_false() {
    let filter = row_filter(
        &dataset(vec![region_policy(Some(roles(&["analyst"])), None)]),
        &user(r#"["manager"]"#, "{}"),
    );
    assert_eq!(filter, Ok(Some(and(boolean(false), boolean(true)))));
}

#[test]
fn e4_6_9a_a_list_attribute_is_kept_as_the_single_item_of_in() {
    let condition = in_list(field("region"), attribute("regions"), false);
    let filter = row_filter(
        &dataset(vec![policy("regions_scope", condition.clone(), None, None)]),
        &user("[]", r#"{"regions": ["EU", "US"]}"#),
    );
    assert_eq!(filter, Ok(Some(and(boolean(true), condition))));
}

#[test]
fn e4_6_9a_an_empty_list_attribute_is_present_with_in_and_not_in() {
    for negated in [false, true] {
        let condition = in_list(field("region"), attribute("regions"), negated);
        let filter = row_filter(
            &dataset(vec![policy("regions_scope", condition.clone(), None, None)]),
            &user("[]", r#"{"regions": []}"#),
        );
        assert_eq!(filter, Ok(Some(and(boolean(true), condition))));
    }
}

#[test]
fn sec_3_5_attribute_values_never_appear_in_the_row_filter() {
    let filter = row_filter(
        &dataset(vec![region_policy(None, None)]),
        &user("[]", r#"{"region": "SECRET-REGION"}"#),
    )
    .expect("a row filter")
    .expect("a policy");
    assert!(!format!("{filter:?}").contains("SECRET-REGION"));
}

#[test]
fn sec_3_4_an_applicable_policy_with_an_absent_attribute_is_esec_02() {
    let filter = row_filter(
        &dataset(vec![region_policy(Some(roles(&["analyst"])), None)]),
        &user(r#"["analyst"]"#, "{}"),
    );
    assert_eq!(
        filter,
        Err(RowFilterError::MissingAttribute {
            attribute: "region".to_owned()
        })
    );
}

#[test]
fn e4_6_9_exempt_and_non_granting_policies_never_fail() {
    let filter = row_filter(
        &dataset(vec![
            region_policy(None, Some(roles(&["auditor"]))),
            team_policy(Some(roles(&["manager"]))),
        ]),
        &user(r#"["auditor"]"#, "{}"),
    );
    assert_eq!(filter, Ok(Some(and(boolean(false), boolean(true)))));
}

#[test]
fn e4_6_9_the_first_missing_attribute_in_policy_then_appearance_order_is_reported() {
    let both = and(
        equals(field("team"), attribute("team")),
        equals(field("region"), attribute("region")),
    );
    let filter = row_filter(
        &dataset(vec![
            policy("first_scope", both, None, None),
            policy("second_scope", attribute("regions"), None, None),
        ]),
        &user("[]", "{}"),
    );
    assert_eq!(
        filter,
        Err(RowFilterError::MissingAttribute {
            attribute: "team".to_owned()
        })
    );
}

#[test]
fn e4_6_9_esec_02_is_located_at_the_attributes_and_names_only_the_attribute() {
    let error = row_filter(&dataset(vec![region_policy(None, None)]), &user("[]", "{}"))
        .expect_err("a missing attribute");
    let diagnostic = error.to_diagnostic().expect("a specification diagnostic");
    assert_eq!(diagnostic.code, "ESEC-02");
    let location = diagnostic.location.expect("a location");
    assert_eq!(location.document, InputDocument::SecurityContext);
    assert_eq!(location.pointer, "/attributes");
    assert!(diagnostic.message.contains("`region`"));
    for hidden in ["region_scope", "ORDERS", "main.orders", "="] {
        assert!(!diagnostic.message.contains(hidden), "{hidden}");
    }
}

#[test]
fn e4_6_2_a_policy_without_a_parsed_condition_is_an_internal_error() {
    let mut unparsed = region_policy(None, None);
    unparsed.condition = None;
    let error =
        row_filter(&dataset(vec![unparsed]), &user("[]", "{}")).expect_err("an unparsed condition");
    assert!(matches!(error, RowFilterError::UnparsedCondition { .. }));
    assert!(error.to_diagnostic().is_none());
}
