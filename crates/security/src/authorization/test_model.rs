//! A small internal model for the unit tests of authorization.

use menura_contracts::Sha256Hash;
use menura_model::expression::{AggregateCall, Expression, Identifier, Reference, SimpleAggregate};
use menura_model::internal_model::{
    AccessMode, Dataset, DatasetAccess, DerivedMetric, Descriptions, Field, FieldRestriction,
    InternalModel, Metric, ModelParts, NormalizedName, Relationship, RelationshipGraph, RoleName,
    SecurityContext, SourceLocation,
};

use crate::context::{AdmittedSecurityContext, admit_security_context};

pub(super) fn name(text: &str) -> NormalizedName {
    NormalizedName::new(text)
}

fn location() -> SourceLocation {
    SourceLocation {
        pointer: String::new(),
        line: 1,
        column: 1,
    }
}

fn reference(qualifier: Option<&str>, field: &str) -> Reference {
    Reference {
        qualifier: qualifier.map(|text| Identifier::regular(text).expect("an identifier")),
        name: Identifier::regular(field).expect("an identifier"),
    }
}

fn sum(qualifier: Option<&str>, field: &str) -> Expression {
    Expression::Aggregate {
        call: AggregateCall::Simple {
            function: SimpleAggregate::Sum,
            argument: Box::new(Expression::Reference(reference(qualifier, field))),
        },
        filter: None,
    }
}

/// A field named `field_name` whose expression is the source column `column`.
fn field(field_name: &str, column: &str) -> Field {
    Field {
        name: name(field_name),
        expression: Some(Expression::Reference(reference(None, column))),
        typed: None,
        is_time: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

pub(super) fn metric(metric_name: &str, expression: Expression) -> Metric {
    Metric {
        name: name(metric_name),
        expression: Some(expression),
        typed: None,
        classification: None,
        declared_additivity: None,
        derived: None,
        context: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

fn roles(names: &[&str]) -> Vec<RoleName> {
    names.iter().map(|role| RoleName::new(role)).collect()
}

fn dataset(dataset_name: &str, access: Option<DatasetAccess>, fields: Vec<Field>) -> Dataset {
    Dataset {
        name: name(dataset_name),
        source: dataset_name.to_lowercase(),
        fields,
        metrics: Vec::new(),
        primary_key: Vec::new(),
        unique_keys: Vec::new(),
        access,
        row_policies: Vec::new(),
        populations: Vec::new(),
        summary: false,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

fn access(mode: AccessMode, fields: Vec<FieldRestriction>) -> DatasetAccess {
    DatasetAccess { mode, fields }
}

/// `ORDERS_BY_COST`, keyed by `COST`, a source column of `ORDERS` that is not a field.
fn cost_keyed_relationship() -> Relationship {
    let mut keyed = relationship("ORDERS_BY_COST", "ORDERS", "CUSTOMERS");
    keyed.from_columns = vec![name("COST")];
    keyed
}

fn relationship(relationship_name: &str, from: &str, to: &str) -> Relationship {
    Relationship {
        name: name(relationship_name),
        from: name(from),
        to: name(to),
        from_columns: vec![name("CUSTOMER_ID")],
        to_columns: vec![name("CUSTOMER_ID")],
        temporal: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

fn orders() -> Dataset {
    let mut orders = dataset(
        "ORDERS",
        Some(access(
            AccessMode::Roles(roles(&["admin", "analyst"])),
            vec![FieldRestriction {
                fields: vec![name("UNIT_COST")],
                roles: roles(&["admin"]),
            }],
        )),
        vec![
            field("AMOUNT", "amount"),
            field("COST_COPY", "COST"),
            field("CUSTOMER_ID", "customer_id"),
            field("UNIT_COST", "cost"),
        ],
    );
    orders.metrics = vec![
        metric("RAW_AMOUNT", sum(Some("source"), "amount")),
        metric("RAW_COST", sum(Some("source"), "COST")),
        metric("TOTAL_COST", sum(None, "cost_copy")),
    ];
    orders
}

fn datasets() -> Vec<Dataset> {
    let open = |mode| Some(access(mode, Vec::new()));
    vec![
        dataset("CLOSED", None, vec![field("VALUE", "value")]),
        dataset(
            "CUSTOMERS",
            open(AccessMode::Authenticated),
            vec![field("CUSTOMER_ID", "customer_id")],
        ),
        orders(),
        dataset(
            "SECRETS",
            open(AccessMode::Roles(roles(&["admin"]))),
            vec![field("CUSTOMER_ID", "customer_id"), field("VALUE", "value")],
        ),
        dataset("SUMMARY", open(AccessMode::Internal), Vec::new()),
    ]
}

fn metrics() -> Vec<Metric> {
    let mut margin = metric(
        "MARGIN",
        Expression::MetricAggregate(reference(None, "revenue")),
    );
    margin.derived = Some(DerivedMetric {
        inputs: vec![name("COST"), name("REVENUE")],
        on_division_by_zero: None,
    });
    vec![
        metric("COST", sum(Some("orders"), "unit_cost")),
        margin,
        metric("REVENUE", sum(Some("orders"), "amount")),
        metric("SECRET_TOTAL", sum(Some("secrets"), "value")),
    ]
}

/// `ORDERS` (roles analyst and admin; `UNIT_COST` restricted to admin, and `COST_COPY` reads
/// its column; dataset metrics `RAW_AMOUNT` and `RAW_COST` sum the source columns `amount`
/// and `COST`, and `TOTAL_COST` sums `COST_COPY`), `CUSTOMERS` (authenticated),
/// `SECRETS` (admin), `CLOSED` (no `access`), `SUMMARY` (internal); metrics `REVENUE`, `COST`,
/// `SECRET_TOTAL` and the derived `MARGIN`.
pub(super) fn model() -> InternalModel {
    let parts = ModelParts {
        name: name("SALES"),
        descriptions: Descriptions::default(),
        datasets: datasets(),
        relationships: vec![
            cost_keyed_relationship(),
            relationship("ORDERS_TO_CUSTOMERS", "ORDERS", "CUSTOMERS"),
            relationship("ORDERS_TO_SECRETS", "ORDERS", "SECRETS"),
        ],
        graph: RelationshipGraph::default(),
        metrics: metrics(),
        security_context: Some(declarations()),
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    InternalModel::new(parts, Sha256Hash::try_from("0".repeat(64)).expect("a hash"))
}

fn declarations() -> SecurityContext {
    SecurityContext {
        roles: roles(&["admin", "analyst"]),
        attributes: Vec::new(),
    }
}

/// The admitted context of a user holding `held` roles.
pub(super) fn user(held: &[&str]) -> AdmittedSecurityContext {
    let roles: Vec<String> = held.iter().map(|role| format!("\"{role}\"")).collect();
    let text = format!(
        r#"{{"contract": "menura.security_context", "contract_version": "1.1.0",
            "subject": "user-1", "roles": [{}], "attributes": {{}}}}"#,
        roles.join(", ")
    );
    admit_security_context(&text, Some(&declarations())).expect("an admitted context")
}
