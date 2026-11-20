//! E2-4.1, E2-4.2, E2-4.3, E2-4.6 and E2-4.8, as the `security` crate decides them.

use menura_contracts::types::audit_record::Access;
use menura_model::internal_model::{AccessMode, FieldPath, RoleName};
use menura_representations::bound_query::{AuthorizationRule, AuthorizedObject, MetricPath};

use super::Authorization;
use super::test_model::{model, name, user};

fn field(dataset: &str, field: &str) -> FieldPath {
    FieldPath {
        dataset: name(dataset),
        field: name(field),
    }
}

fn semantic(metric: &str) -> MetricPath {
    MetricPath::SemanticModel(name(metric))
}

#[test]
fn e2_4_1_a_dataset_is_granted_by_its_access_rule() {
    let model = model();
    let analyst = user(&["analyst"]);
    let authorization = Authorization::new(&model, &analyst);
    let orders = authorization.dataset(&name("ORDERS"));
    assert_eq!(orders.object, AuthorizedObject::Dataset(name("ORDERS")));
    assert_eq!(orders.access, Access::Granted);
    assert_eq!(
        orders.rule,
        AuthorizationRule::DatasetAccess(AccessMode::Roles(vec![
            RoleName::new("admin"),
            RoleName::new("analyst"),
        ]))
    );
    assert_eq!(
        authorization.dataset(&name("CUSTOMERS")).access,
        Access::Granted
    );
    assert_eq!(
        authorization.dataset(&name("SECRETS")).access,
        Access::Denied
    );
}

#[test]
fn e2_4_1_a_dataset_without_access_is_denied() {
    let model = model();
    let admin = user(&["admin"]);
    let closed = Authorization::new(&model, &admin).dataset(&name("CLOSED"));
    assert_eq!(closed.access, Access::Denied);
    assert_eq!(closed.rule, AuthorizationRule::NoDatasetAccess);
}

#[test]
fn e2_4_1_undeclared_roles_grant_nothing() {
    let model = model();
    let stranger = user(&["ADMIN", "auditor"]);
    let authorization = Authorization::new(&model, &stranger);
    assert_eq!(
        authorization.dataset(&name("SECRETS")).access,
        Access::Denied
    );
    assert_eq!(
        authorization.dataset(&name("ORDERS")).access,
        Access::Denied
    );
    assert_eq!(
        authorization.dataset(&name("CUSTOMERS")).access,
        Access::Granted
    );
}

#[test]
fn e2_4_6_an_internal_dataset_is_denied_to_every_user() {
    let model = model();
    let admin = user(&["admin", "analyst"]);
    let summary = Authorization::new(&model, &admin).dataset(&name("SUMMARY"));
    assert_eq!(summary.access, Access::Denied);
    assert_eq!(
        summary.rule,
        AuthorizationRule::DatasetAccess(AccessMode::Internal)
    );
}

#[test]
fn e2_4_2_a_restricted_field_needs_one_of_its_roles() {
    let model = model();
    let analyst = user(&["analyst"]);
    let admin = user(&["admin"]);
    let unit_cost = field("ORDERS", "UNIT_COST");
    let denied = Authorization::new(&model, &analyst).field(&unit_cost);
    assert_eq!(denied.access, Access::Denied);
    assert_eq!(
        denied.rule,
        AuthorizationRule::FieldRestriction(vec![RoleName::new("admin")])
    );
    assert_eq!(
        Authorization::new(&model, &admin).field(&unit_cost).access,
        Access::Granted
    );
}

#[test]
fn e2_4_2_a_field_reading_a_protected_column_is_restricted_too() {
    let model = model();
    let analyst = user(&["analyst"]);
    let authorization = Authorization::new(&model, &analyst);
    assert_eq!(
        authorization.field(&field("ORDERS", "COST_COPY")).access,
        Access::Denied
    );
    let amount = authorization.field(&field("ORDERS", "AMOUNT"));
    assert_eq!(amount.access, Access::Granted);
    assert_eq!(amount.rule, AuthorizationRule::NoFieldRestriction);
}

#[test]
fn e2_4_3_a_metric_reading_a_restricted_field_is_denied_without_its_role() {
    let model = model();
    let analyst = user(&["analyst"]);
    let authorization = Authorization::new(&model, &analyst);
    let cost = authorization.metric(&semantic("COST"));
    assert_eq!(cost.object, AuthorizedObject::Metric(semantic("COST")));
    assert_eq!(cost.access, Access::Denied);
    assert_eq!(cost.rule, AuthorizationRule::MetricComponents);
    let total_cost = MetricPath::Dataset {
        dataset: name("ORDERS"),
        metric: name("TOTAL_COST"),
    };
    assert_eq!(authorization.metric(&total_cost).access, Access::Denied);
    assert_eq!(
        authorization.metric(&semantic("REVENUE")).access,
        Access::Granted
    );
}

#[test]
fn e2_4_3_a_metric_reading_an_unauthorized_dataset_is_denied() {
    let model = model();
    let analyst = user(&["analyst"]);
    let admin = user(&["admin"]);
    let secret = semantic("SECRET_TOTAL");
    assert_eq!(
        Authorization::new(&model, &analyst).metric(&secret).access,
        Access::Denied
    );
    assert_eq!(
        Authorization::new(&model, &admin).metric(&secret).access,
        Access::Granted
    );
}

#[test]
fn e2_4_3_a_derived_metric_needs_every_input() {
    let model = model();
    let analyst = user(&["analyst"]);
    let admin = user(&["admin"]);
    let margin = semantic("MARGIN");
    assert_eq!(
        Authorization::new(&model, &analyst).metric(&margin).access,
        Access::Denied
    );
    assert_eq!(
        Authorization::new(&model, &admin).metric(&margin).access,
        Access::Granted
    );
}

#[test]
fn e2_4_8_the_subgraph_holds_admitted_datasets_and_relationships_between_them() {
    let model = model();
    let analyst = user(&["analyst"]);
    let subgraph = Authorization::new(&model, &analyst).subgraph();
    assert_eq!(subgraph.datasets(), &[name("CUSTOMERS"), name("ORDERS")]);
    assert_eq!(
        subgraph.relationships(),
        &[name("ORDERS_BY_COST"), name("ORDERS_TO_CUSTOMERS")]
    );
    let admin = user(&["admin"]);
    let subgraph = Authorization::new(&model, &admin).subgraph();
    assert_eq!(
        subgraph.relationships(),
        &[
            name("ORDERS_BY_COST"),
            name("ORDERS_TO_CUSTOMERS"),
            name("ORDERS_TO_SECRETS"),
        ]
    );
}

#[test]
fn e2_4_3_a_source_column_read_is_protected_by_the_restrictions_of_that_column() {
    let model = model();
    let analyst = user(&["analyst"]);
    let admin = user(&["admin"]);
    let scoped = |metric: &str| MetricPath::Dataset {
        dataset: name("ORDERS"),
        metric: name(metric),
    };
    let raw_cost = scoped("RAW_COST");
    assert_eq!(
        Authorization::new(&model, &analyst)
            .metric(&raw_cost)
            .access,
        Access::Denied
    );
    assert_eq!(
        Authorization::new(&model, &admin).metric(&raw_cost).access,
        Access::Granted
    );
    assert_eq!(
        Authorization::new(&model, &analyst)
            .metric(&scoped("RAW_AMOUNT"))
            .access,
        Access::Granted
    );
}

#[test]
fn e2_4_2_a_key_column_that_is_not_a_field_is_protected_by_the_restrictions_of_its_column() {
    let model = model();
    let analyst = user(&["analyst"]);
    let admin = user(&["admin"]);
    assert!(!Authorization::new(&model, &analyst).key_column(&name("ORDERS"), &name("COST")));
    assert!(Authorization::new(&model, &admin).key_column(&name("ORDERS"), &name("COST")));
}

#[test]
fn e2_4_1_a_metric_uses_every_dataset_it_and_its_inputs_read() {
    let model = model();
    let analyst = user(&["analyst"]);
    let authorization = Authorization::new(&model, &analyst);
    assert_eq!(
        authorization.metric_datasets(&semantic("MARGIN")),
        vec![name("ORDERS")]
    );
    assert_eq!(
        authorization.metric_datasets(&semantic("SECRET_TOTAL")),
        vec![name("SECRETS")]
    );
}
