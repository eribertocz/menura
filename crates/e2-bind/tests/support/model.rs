//! The internal model the tests of E2 bind against.

use menura_contracts::Sha256Hash;
use menura_contracts::types::common::{LogicalType, TimestampPrecision};
use menura_model::expression::{AggregateCall, Expression, Identifier, Reference, SimpleAggregate};
use menura_model::internal_model::{
    AccessMode, Additivity, Dataset, DatasetAccess, Descriptions, DiagnosticCode, Field,
    FieldRestriction, InternalModel, Metric, MetricClassification, ModelParts, NormalizedName,
    Relationship, RelationshipGraph, RoleName, SecurityContext, SourceLocation, TypedValue,
    ValueType,
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

/// The classification of a metric whose components are all on `home`.
pub fn classification(home: &[&str]) -> MetricClassification {
    MetricClassification {
        components: Vec::new(),
        inputs: Vec::new(),
        home_datasets: home.iter().map(|dataset| name(dataset)).collect(),
        additivity: Additivity::Additive,
        recomposable: true,
    }
}

fn metric(metric_name: &str, expression: Expression, home: &str) -> Metric {
    Metric {
        name: name(metric_name),
        expression: Some(expression),
        typed: None,
        classification: Some(classification(&[home])),
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
        temporal: None,
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
    if let Some(ordered_at) = orders
        .fields
        .iter_mut()
        .find(|field| field.name == name("ORDERED_AT"))
    {
        ordered_at.is_time = Some(true);
        ordered_at.typed = Some(TypedValue {
            value_type: ValueType::Logical(LogicalType::Timestamp {
                precision: TimestampPrecision::Us,
                with_time_zone: true,
            }),
            nullable: true,
        });
    }
    for field in &mut orders.fields {
        if field.name == name("REGION") {
            field.typed = Some(TypedValue {
                value_type: ValueType::Logical(LogicalType::Text),
                nullable: true,
            });
        }
    }
    let mut unit_cost = field("UNIT_COST");
    unit_cost.expression = Some(column("cost"));
    orders.fields.push(unit_cost);
    orders.metrics = vec![
        metric("ORDER_TOTAL", sum(None, "amount"), "ORDERS"),
        metric("TOTAL_COST", sum(None, "unit_cost"), "ORDERS"),
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

/// The model of [`model`] after `edit`.
pub fn model_with(edit: impl FnOnce(&mut ModelParts)) -> InternalModel {
    let mut parts = model().parts().clone();
    edit(&mut parts);
    InternalModel::new(parts, Sha256Hash::try_from("0".repeat(64)).expect("a hash"))
}

/// `CUSTOMERS` (authenticated: `Segment`, `CUSTOMER_ID`, `REGION`), `ORDERS` (admin and
/// analyst: `AMOUNT`, `CUSTOMER_ID`, `ORDERED_AT` (a time field, a timestamp with time zone),
/// `REGION` (text), and `UNIT_COST`, which reads the
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
            metric("COST", sum(Some("orders"), "unit_cost"), "ORDERS"),
            metric("REVENUE", sum(Some("orders"), "amount"), "ORDERS"),
            metric("SECRET_TOTAL", sum(Some("secrets"), "value"), "SECRETS"),
        ],
        security_context: Some(declarations()),
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    InternalModel::new(parts, Sha256Hash::try_from("0".repeat(64)).expect("a hash"))
}

/// Marks the field `dataset.field` of `parts` unusable with `codes`.
pub fn mark_field(parts: &mut ModelParts, dataset: &str, field: &str, codes: &[&str]) {
    let found = parts
        .datasets
        .iter_mut()
        .find(|candidate| candidate.name == name(dataset))
        .and_then(|found| {
            found
                .fields
                .iter_mut()
                .find(|candidate| candidate.name == name(field))
        })
        .expect("a field of the model");
    found.query_errors = codes.iter().map(|code| DiagnosticCode::new(code)).collect();
}

/// The metric of the semantic model named `metric`.
pub fn semantic_metric<'parts>(parts: &'parts mut ModelParts, metric: &str) -> &'parts mut Metric {
    parts
        .metrics
        .iter_mut()
        .find(|candidate| candidate.name == name(metric))
        .expect("a metric of the model")
}

/// Marks the metric `metric` of the semantic model unusable with `codes`.
pub fn mark_metric(parts: &mut ModelParts, metric: &str, codes: &[&str]) {
    semantic_metric(parts, metric).query_errors =
        codes.iter().map(|code| DiagnosticCode::new(code)).collect();
}

/// `parts` with the edge `ORDERS` → `CUSTOMERS` and a row policy on `CUSTOMERS` for `admin`.
pub fn customers_with_a_row_policy(parts: &mut ModelParts) {
    set_edges(parts, &[("ORDERS", "CUSTOMERS")]);
    add_row_policy(parts, "CUSTOMERS");
}

/// Sets the many-to-one edges of the graph of `parts` to `edges`, each `(from, to)`.
pub fn set_edges(parts: &mut ModelParts, edges: &[(&str, &str)]) {
    parts.graph.edges = edges
        .iter()
        .map(|(from, to)| menura_model::internal_model::GraphEdge {
            relationship: name(&format!("{from}_TO_{to}")),
            from: name(from),
            to: name(to),
            one_to_one: false,
        })
        .collect();
}

/// Adds a row policy for `admin` to `dataset`.
pub fn add_row_policy(parts: &mut ModelParts, dataset: &str) {
    let found = parts
        .datasets
        .iter_mut()
        .find(|candidate| candidate.name == name(dataset))
        .expect("a dataset of the model");
    found.row_policies = vec![menura_model::internal_model::RowPolicy {
        name: "admins_only".to_owned(),
        condition: None,
        roles: Some(vec![RoleName::new("admin")]),
        exempt_roles: None,
        location: SourceLocation {
            pointer: String::new(),
            line: 1,
            column: 1,
        },
    }];
}

/// Adds to `parts` the metric `metric` of the semantic model, `SUM(dataset.field)`, whose
/// home dataset is `dataset`.
pub fn add_semantic_metric(parts: &mut ModelParts, metric_name: &str, dataset: &str, field: &str) {
    let expression = sum(Some(&dataset.to_lowercase()), &field.to_lowercase());
    parts.metrics.push(metric(metric_name, expression, dataset));
    parts
        .metrics
        .sort_by(|left, right| left.name.cmp(&right.name));
}
