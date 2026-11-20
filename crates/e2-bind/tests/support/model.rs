//! The internal model the tests of E2 bind against.

use menura_contracts::Sha256Hash;
use menura_model::expression::{AggregateCall, Expression, Identifier, Reference, SimpleAggregate};
use menura_model::internal_model::{
    AccessMode, Dataset, DatasetAccess, Descriptions, Field, FieldRestriction, InternalModel,
    Metric, ModelParts, NormalizedName, Relationship, RelationshipGraph, RoleName, SecurityContext,
    SourceLocation,
};

/// A normalized name.
pub fn name(text: &str) -> NormalizedName {
    NormalizedName::new(text)
}

fn location() -> SourceLocation {
    SourceLocation {
        pointer: String::new(),
        line: 1,
        column: 1,
    }
}

fn column(text: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::regular(text).expect("an identifier"),
    })
}

fn field(field_name: &str) -> Field {
    Field {
        name: name(field_name),
        expression: Some(column(&field_name.to_lowercase())),
        typed: None,
        is_time: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

fn sum(qualifier: Option<&str>, field: &str) -> Expression {
    let reference = Reference {
        qualifier: qualifier.map(|text| Identifier::regular(text).expect("an identifier")),
        name: Identifier::regular(field).expect("an identifier"),
    };
    Expression::Aggregate {
        call: AggregateCall::Simple {
            function: SimpleAggregate::Sum,
            argument: Box::new(Expression::Reference(reference)),
        },
        filter: None,
    }
}

fn metric(metric_name: &str, expression: Expression) -> Metric {
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

fn dataset(dataset_name: &str, access: DatasetAccess, fields: &[&str]) -> Dataset {
    Dataset {
        name: name(dataset_name),
        source: dataset_name.to_lowercase(),
        fields: fields.iter().map(|text| field(text)).collect(),
        metrics: Vec::new(),
        primary_key: Vec::new(),
        unique_keys: Vec::new(),
        access: Some(access),
        row_policies: Vec::new(),
        populations: Vec::new(),
        summary: false,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

fn access(mode: AccessMode) -> DatasetAccess {
    DatasetAccess {
        mode,
        fields: Vec::new(),
    }
}

fn relationship(relationship_name: &str, from: &str, to: &str) -> Relationship {
    keyed_relationship(relationship_name, from, to, "CUSTOMER_ID")
}

/// The relationship from `from` to `to` whose `from_columns` is `key`.
fn keyed_relationship(relationship_name: &str, from: &str, to: &str, key: &str) -> Relationship {
    Relationship {
        name: name(relationship_name),
        from: name(from),
        to: name(to),
        from_columns: vec![name(key)],
        to_columns: vec![name("CUSTOMER_ID")],
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

fn orders() -> Dataset {
    let mut access = access(AccessMode::Roles(roles(&["admin", "analyst"])));
    access.fields = vec![FieldRestriction {
        fields: vec![name("UNIT_COST")],
        roles: roles(&["admin"]),
    }];
    let mut orders = dataset(
        "ORDERS",
        access,
        &["AMOUNT", "CUSTOMER_ID", "ORDERED_AT", "REGION"],
    );
    let mut unit_cost = field("UNIT_COST");
    unit_cost.expression = Some(column("cost"));
    orders.fields.push(unit_cost);
    orders.metrics = vec![
        metric("ORDER_TOTAL", sum(None, "amount")),
        metric("TOTAL_COST", sum(None, "unit_cost")),
    ];
    orders
}

fn datasets() -> Vec<Dataset> {
    let mut customers = dataset(
        "CUSTOMERS",
        access(AccessMode::Authenticated),
        &["CUSTOMER_ID", "REGION"],
    );
    customers.fields.push(field("Segment"));
    let mut summary = dataset("ORDERS_SUMMARY", access(AccessMode::Internal), &["REGION"]);
    summary.summary = true;
    vec![
        customers,
        orders(),
        summary,
        dataset(
            "SECRETS",
            access(AccessMode::Roles(roles(&["admin"]))),
            &["CUSTOMER_ID", "VALUE"],
        ),
    ]
}

/// The declared roles and attributes of the model.
pub fn declarations() -> SecurityContext {
    SecurityContext {
        roles: roles(&["admin", "analyst"]),
        attributes: Vec::new(),
    }
}

/// `CUSTOMERS` (authenticated: `Segment`, `CUSTOMER_ID`, `REGION`), `ORDERS` (admin and
/// analyst: `AMOUNT`, `CUSTOMER_ID`, `ORDERED_AT`, `REGION`, and `UNIT_COST`, which reads the
/// source column `cost` and is restricted to admin; dataset metrics `ORDER_TOTAL` and `TOTAL_COST`), `ORDERS_SUMMARY` (internal summary),
/// `SECRETS` (admin: `CUSTOMER_ID`, `VALUE`); metrics `COST`, `REVENUE` and `SECRET_TOTAL`;
/// relationships `ORDERS_BY_COST` (keyed by the source column `COST`), `ORDERS_BY_UNIT_COST`
/// (keyed by the field `UNIT_COST`), `ORDERS_TO_CUSTOMERS` and `ORDERS_TO_SECRETS`.
pub fn model() -> InternalModel {
    let parts = ModelParts {
        name: name("SALES"),
        descriptions: Descriptions::default(),
        datasets: datasets(),
        relationships: vec![
            keyed_relationship("ORDERS_BY_COST", "ORDERS", "CUSTOMERS", "COST"),
            keyed_relationship("ORDERS_BY_UNIT_COST", "ORDERS", "CUSTOMERS", "UNIT_COST"),
            relationship("ORDERS_TO_CUSTOMERS", "ORDERS", "CUSTOMERS"),
            relationship("ORDERS_TO_SECRETS", "ORDERS", "SECRETS"),
        ],
        graph: RelationshipGraph::default(),
        metrics: vec![
            metric("COST", sum(Some("orders"), "unit_cost")),
            metric("REVENUE", sum(Some("orders"), "amount")),
            metric("SECRET_TOTAL", sum(Some("secrets"), "value")),
        ],
        security_context: Some(declarations()),
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    InternalModel::new(parts, Sha256Hash::try_from("0".repeat(64)).expect("a hash"))
}
