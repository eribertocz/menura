//! E2-4 and PLN-5.2: every name is authorized as soon as it resolves, and an unauthorized one
//! is reported exactly as a nonexistent one.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_contracts::types::audit_record::Access;
use menura_e2_bind::BindCode;
use menura_model::internal_model::FieldPath;
use menura_representations::bound_query::{AuthorizedObject, MetricPath};
use menura_representations::query_tree::QueryTreeParts;

use support::model::name;
use support::query::{
    column, dimension, equals_text, filter, metric, path, time_grain, tree, user,
};
use support::{diagnostic, first_error, run};

const ANALYST: &[&str] = &["analyst"];
const ADMIN: &[&str] = &["admin"];

fn field(dataset: &str, field: &str) -> FieldPath {
    FieldPath {
        dataset: name(dataset),
        field: name(field),
    }
}

/// Asserts that the diagnostics of `unauthorized` and `unknown` are the same, apart from the
/// name as written.
fn assert_same_diagnostic(unauthorized: &str, unknown: &str, as_metric: bool) {
    let query = |text: &str| {
        if as_metric {
            tree(vec![], vec![metric(0, text)], QueryTreeParts::default())
        } else {
            tree(vec![dimension(0, text)], vec![], QueryTreeParts::default())
        }
    };
    let denied = diagnostic(&query(unauthorized), ANALYST);
    let missing = diagnostic(&query(unknown), ANALYST);
    assert_eq!(denied.code, BindCode::NameNotFound);
    assert_eq!(denied.code, missing.code);
    assert_eq!(denied.location, missing.location);
    assert_eq!(
        denied.message.replace(unauthorized, "<name>"),
        missing.message.replace(unknown, "<name>")
    );
}

#[test]
fn e2_4_4_an_unauthorized_dataset_is_reported_as_a_nonexistent_one() {
    assert_same_diagnostic("secrets.value", "secretz.value", false);
}

#[test]
fn e2_4_4_an_unauthorized_field_is_reported_as_a_nonexistent_one() {
    assert_same_diagnostic("orders.unit_cost", "orders.unit_kost", false);
}

#[test]
fn e2_4_4_an_unauthorized_metric_is_reported_as_a_nonexistent_one() {
    assert_same_diagnostic("secret_total", "secret_totals", true);
}

#[test]
fn pln_5_2_the_message_never_names_the_model_element_or_its_roles() {
    let query = tree(
        vec![dimension(0, "orders.unit_cost")],
        vec![],
        QueryTreeParts::default(),
    );
    let message = diagnostic(&query, ANALYST).message;
    assert!(message.contains("orders.unit_cost"), "{message}");
    for hidden in [
        "admin",
        "UNIT_COST",
        "restrict",
        "authoriz",
        "access",
        "permission",
    ] {
        assert!(!message.contains(hidden), "{message}");
    }
}

#[test]
fn e2_4_1_a_used_dataset_is_authorized_by_its_access_rule() {
    let query = tree(
        vec![dimension(0, "secrets.value")],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(first_error(&query, ANALYST).0, BindCode::NameNotFound);
    assert!(run(&query, ADMIN).is_ok());
}

#[test]
fn e2_4_1_a_dataset_named_by_a_path_is_authorized() {
    let parts = QueryTreeParts {
        paths: vec![path(
            "/query/nonportable_extensions/0/data/paths/0",
            "orders",
            "secrets",
            &["orders_to_secrets"],
        )],
        ..QueryTreeParts::default()
    };
    let query = tree(vec![dimension(0, "orders.region")], vec![], parts);
    assert_eq!(
        first_error(&query, ANALYST),
        (
            BindCode::NameNotFound,
            "/query/nonportable_extensions/0/data/paths/0".to_owned()
        )
    );
    assert!(run(&query, ADMIN).is_ok());
}

#[test]
fn e2_4_1_a_filter_field_is_authorized() {
    let parts = QueryTreeParts {
        filters: vec![filter(
            "/query/filter",
            equals_text(column("secrets.value")),
        )],
        ..QueryTreeParts::default()
    };
    let query = tree(vec![dimension(0, "orders.region")], vec![], parts);
    assert_eq!(
        first_error(&query, ANALYST),
        (BindCode::NameNotFound, "/query/filter".to_owned())
    );
}

#[test]
fn e2_4_2_a_field_restricted_to_a_role_the_user_lacks_is_not_found() {
    let query = tree(
        vec![dimension(0, "orders.unit_cost")],
        vec![],
        QueryTreeParts::default(),
    );
    assert_eq!(
        first_error(&query, ANALYST),
        (BindCode::NameNotFound, "/query/dimensions/0".to_owned())
    );
    assert!(run(&query, ADMIN).is_ok());
}

#[test]
fn e2_4_3_a_metric_reading_a_restricted_field_is_not_found() {
    for text in ["cost", "orders.total_cost"] {
        let query = tree(vec![], vec![metric(0, text)], QueryTreeParts::default());
        assert_eq!(
            first_error(&query, ANALYST),
            (BindCode::NameNotFound, "/query/metrics/0".to_owned()),
            "{text}"
        );
        assert!(run(&query, ADMIN).is_ok(), "{text}");
    }
}

#[test]
fn e2_4_5_datasets_the_query_does_not_use_need_no_authorization() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    let authorized = run(&query, ANALYST).expect("authorized");
    let named_secrets = authorized
        .authorization()
        .iter()
        .any(|decision| decision.object == AuthorizedObject::Dataset(name("SECRETS")));
    assert!(!named_secrets);
}

#[test]
fn e2_4_6_an_internal_summary_dataset_can_never_be_named() {
    let query = tree(
        vec![dimension(0, "orders_summary.region")],
        vec![],
        QueryTreeParts::default(),
    );
    for roles in [ANALYST, ADMIN, &["admin", "analyst"][..]] {
        assert_eq!(first_error(&query, roles).0, BindCode::NameNotFound);
    }
}

#[test]
fn e2_4_7_every_decision_is_recorded_in_the_order_taken() {
    let parts = QueryTreeParts {
        filters: vec![filter(
            "/query/filter",
            equals_text(column("customers.region")),
        )],
        ..QueryTreeParts::default()
    };
    let query = tree(
        vec![dimension(0, "orders.region"), dimension(1, "orders.amount")],
        vec![metric(0, "revenue"), metric(1, "orders.order_total")],
        parts,
    );
    let authorized = run(&query, ANALYST).expect("authorized");
    let objects: Vec<&AuthorizedObject> = authorized
        .authorization()
        .iter()
        .map(|decision| &decision.object)
        .collect();
    let expected = [
        AuthorizedObject::Dataset(name("ORDERS")),
        AuthorizedObject::Field(field("ORDERS", "REGION")),
        AuthorizedObject::Field(field("ORDERS", "AMOUNT")),
        AuthorizedObject::Metric(MetricPath::SemanticModel(name("REVENUE"))),
        AuthorizedObject::Metric(MetricPath::Dataset {
            dataset: name("ORDERS"),
            metric: name("ORDER_TOTAL"),
        }),
        AuthorizedObject::Dataset(name("CUSTOMERS")),
        AuthorizedObject::Field(field("CUSTOMERS", "REGION")),
    ];
    assert_eq!(objects, expected.iter().collect::<Vec<_>>());
    assert!(
        authorized
            .authorization()
            .iter()
            .all(|decision| decision.access == Access::Granted)
    );
}

#[test]
fn e2_4_8_the_authorized_subgraph_holds_only_admitted_datasets() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts::default(),
    );
    let analyst = run(&query, ANALYST).expect("authorized");
    assert_eq!(
        analyst.subgraph().datasets(),
        &[name("CUSTOMERS"), name("ORDERS")]
    );
    let admin = run(&query, ADMIN).expect("authorized");
    assert_eq!(
        admin.subgraph().datasets(),
        &[name("CUSTOMERS"), name("ORDERS"), name("SECRETS")]
    );
}

#[test]
fn e2_4_8_the_authorized_subgraph_excludes_relationships_keyed_by_restricted_fields() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts::default(),
    );
    let analyst = run(&query, ANALYST).expect("authorized");
    assert_eq!(
        analyst.subgraph().relationships(),
        &[name("ORDERS_TO_CUSTOMERS")]
    );
    let admin = run(&query, ADMIN).expect("authorized");
    assert_eq!(
        admin.subgraph().relationships(),
        &[
            name("ORDERS_BY_COST"),
            name("ORDERS_BY_UNIT_COST"),
            name("ORDERS_TO_CUSTOMERS"),
            name("ORDERS_TO_SECRETS"),
        ]
    );
}

#[test]
fn e2_4_7_the_subgraph_records_no_decision_on_fields_the_query_does_not_name() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts::default(),
    );
    let analyst = run(&query, ANALYST).expect("authorized");
    let unit_cost = AuthorizedObject::Field(field("ORDERS", "UNIT_COST"));
    assert!(
        analyst
            .authorization()
            .iter()
            .all(|decision| decision.object != unit_cost)
    );
}

fn one_path(relationship: &str) -> menura_representations::query_tree::QueryTree {
    let parts = QueryTreeParts {
        paths: vec![path(
            "/query/nonportable_extensions/0/data/paths/0",
            "orders",
            "customers",
            &[relationship],
        )],
        ..QueryTreeParts::default()
    };
    tree(vec![dimension(0, "customers.region")], vec![], parts)
}

#[test]
fn e2_4_8_a_relationship_with_an_unauthorized_end_is_not_found() {
    let query = one_path("orders_to_secrets");
    assert_eq!(
        first_error(&query, ANALYST),
        (
            BindCode::NameNotFound,
            "/query/nonportable_extensions/0/data/paths/0".to_owned()
        )
    );
    assert!(run(&query, ADMIN).is_ok());
}

#[test]
fn e2_4_2_a_relationship_keyed_by_a_restricted_field_is_not_found() {
    let query = one_path("orders_by_unit_cost");
    assert_eq!(first_error(&query, ANALYST).0, BindCode::NameNotFound);
    assert!(run(&query, ADMIN).is_ok());
}

#[test]
fn e2_4_2_a_relationship_keyed_by_a_protected_source_column_is_not_found() {
    let query = one_path("orders_by_cost");
    assert_eq!(first_error(&query, ANALYST).0, BindCode::NameNotFound);
    assert!(run(&query, ADMIN).is_ok());
}

#[test]
fn e2_4_7_a_metric_records_the_datasets_it_reads_before_itself() {
    let query = tree(
        vec![],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    let authorized = run(&query, ANALYST).expect("authorized");
    let objects: Vec<&AuthorizedObject> = authorized
        .authorization()
        .iter()
        .map(|decision| &decision.object)
        .collect();
    let expected = [
        AuthorizedObject::Dataset(name("ORDERS")),
        AuthorizedObject::Metric(MetricPath::SemanticModel(name("REVENUE"))),
    ];
    assert_eq!(objects, expected.iter().collect::<Vec<_>>());
}

#[test]
fn e2_4_1_a_time_grain_field_is_authorized() {
    let parts = QueryTreeParts {
        time_grains: vec![time_grain(
            "/query/nonportable_extensions/0/data/time_grains/0",
            "orders.unit_cost",
            menura_contracts::types::common::Grain::Month,
        )],
        ..QueryTreeParts::default()
    };
    let query = tree(vec![dimension(0, "orders.region")], vec![], parts);
    assert_eq!(
        first_error(&query, ANALYST),
        (
            BindCode::NameNotFound,
            "/query/nonportable_extensions/0/data/time_grains/0".to_owned()
        )
    );
}

#[test]
fn e2_4_7_the_decisions_before_a_denied_name_are_recorded_with_the_denial() {
    let query = tree(
        vec![
            dimension(0, "orders.region"),
            dimension(1, "orders.unit_cost"),
        ],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    let decisions =
        menura_e2_bind::authorization_decisions(&query, &support::model::model(), &user(ANALYST));
    let recorded: Vec<(&AuthorizedObject, Access)> = decisions
        .iter()
        .map(|decision| (&decision.object, decision.access))
        .collect();
    let expected = [
        (AuthorizedObject::Dataset(name("ORDERS")), Access::Granted),
        (
            AuthorizedObject::Field(field("ORDERS", "REGION")),
            Access::Granted,
        ),
        (
            AuthorizedObject::Field(field("ORDERS", "UNIT_COST")),
            Access::Denied,
        ),
    ];
    let expected: Vec<(&AuthorizedObject, Access)> = expected
        .iter()
        .map(|(object, access)| (object, *access))
        .collect();
    assert_eq!(recorded, expected);
}

#[test]
fn e2_4_7_the_decisions_of_an_authorized_query_are_those_of_its_bound_query() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    let decisions =
        menura_e2_bind::authorization_decisions(&query, &support::model::model(), &user(ANALYST));
    let authorized = run(&query, ANALYST).expect("authorized");
    assert_eq!(decisions, authorized.authorization());
}
