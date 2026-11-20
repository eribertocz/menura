//! The documents of datasets, fields, populations, relationships and security.

use menura_model::internal_model::{
    AccessMode, AttributeType, Dataset, DatasetAccess, Descriptions, Field, Population,
    PopulationMode, Relationship, RoleName, RowPolicy, ScalarType, SecurityContext,
};
use serde_json::Value as Json;

use super::{codes, json, metric, name, names, object, serialized, text, tree, typed};

/// `description`, `label` and `ai_context`, as written.
pub(in super::super) fn descriptions(descriptions: &Descriptions) -> Json {
    object([
        (
            "description",
            descriptions.description.as_deref().map_or(Json::Null, text),
        ),
        (
            "label",
            descriptions.label.as_deref().map_or(Json::Null, text),
        ),
        (
            "ai_context",
            descriptions.ai_context.as_ref().map_or(Json::Null, json),
        ),
    ])
}

/// What a check or a summary hash reads of a field: its name, normalized tree and logical type
/// (CHK-3, E0H-3.3).
pub(in super::super) fn field_definition(field: &Field) -> Json {
    object([
        ("name", name(&field.name)),
        ("expression", tree(field.expression.as_ref())),
        ("type", typed(field.typed.as_ref())),
    ])
}

fn field(field: &Field) -> Json {
    object([
        ("definition", field_definition(field)),
        ("is_time", field.is_time.map_or(Json::Null, Json::Bool)),
        ("descriptions", descriptions(&field.descriptions)),
        ("query_errors", codes(&field.query_errors)),
    ])
}

/// The whole meaning of a dataset.
pub(in super::super) fn dataset(dataset: &Dataset) -> Json {
    object([
        ("name", name(&dataset.name)),
        ("source", text(&dataset.source)),
        (
            "fields",
            Json::Array(dataset.fields.iter().map(field).collect()),
        ),
        (
            "metrics",
            Json::Array(dataset.metrics.iter().map(metric).collect()),
        ),
        ("primary_key", names(&dataset.primary_key)),
        (
            "unique_keys",
            Json::Array(dataset.unique_keys.iter().map(names).collect()),
        ),
        ("access", dataset.access.as_ref().map_or(Json::Null, access)),
        (
            "row_policies",
            Json::Array(dataset.row_policies.iter().map(row_policy).collect()),
        ),
        (
            "populations",
            Json::Array(dataset.populations.iter().map(population).collect()),
        ),
        ("summary", Json::Bool(dataset.summary)),
        ("descriptions", descriptions(&dataset.descriptions)),
        ("query_errors", codes(&dataset.query_errors)),
    ])
}

/// The normalized population extension.
pub(in super::super) fn population(population: &Population) -> Json {
    let mode = match &population.mode {
        PopulationMode::Validity {
            valid_from,
            valid_to,
            partial_period,
        } => object([
            ("mode", text("validity")),
            ("valid_from", name(valid_from)),
            ("valid_to", name(valid_to)),
            ("partial_period", text(partial_period.as_str())),
        ]),
        PopulationMode::Periodic { period, grain } => object([
            ("mode", text("periodic")),
            ("period", name(period)),
            ("grain", serialized(grain)),
        ]),
    };
    object([
        ("name", name(&population.name)),
        ("entity", names(&population.entity)),
        ("mode", mode),
        ("required_when", tree(population.required_when.as_ref())),
    ])
}

fn roles(roles: &[RoleName]) -> Json {
    Json::Array(roles.iter().map(|role| text(role.as_str())).collect())
}

fn access(access: &DatasetAccess) -> Json {
    let mode = match &access.mode {
        AccessMode::Authenticated => object([("mode", text("authenticated"))]),
        AccessMode::Roles(list) => object([("mode", text("roles")), ("roles", roles(list))]),
        AccessMode::Internal => object([("mode", text("internal"))]),
    };
    let fields = access
        .fields
        .iter()
        .map(|restriction| {
            object([
                ("fields", names(&restriction.fields)),
                ("roles", roles(&restriction.roles)),
            ])
        })
        .collect();
    object([("mode", mode), ("fields", Json::Array(fields))])
}

fn row_policy(policy: &RowPolicy) -> Json {
    object([
        ("name", text(&policy.name)),
        ("condition", tree(policy.condition.as_ref())),
        ("roles", policy.roles.as_deref().map_or(Json::Null, roles)),
        (
            "exempt_roles",
            policy.exempt_roles.as_deref().map_or(Json::Null, roles),
        ),
    ])
}

/// What a relationship check reads of the relationship: `from`, `to` and the column pairs
/// (CHK-3).
pub(in super::super) fn relationship_definition(relationship: &Relationship) -> Json {
    object([
        ("from", name(&relationship.from)),
        ("to", name(&relationship.to)),
        ("from_columns", names(&relationship.from_columns)),
        ("to_columns", names(&relationship.to_columns)),
    ])
}

/// The whole meaning of a relationship.
pub(in super::super) fn relationship(relationship: &Relationship) -> Json {
    object([
        ("name", name(&relationship.name)),
        ("definition", relationship_definition(relationship)),
        ("descriptions", descriptions(&relationship.descriptions)),
        ("query_errors", codes(&relationship.query_errors)),
    ])
}

/// The `security_context`.
pub(in super::super) fn security_context(context: &SecurityContext) -> Json {
    let attributes = context
        .attributes
        .iter()
        .map(|attribute| {
            object([
                ("name", text(&attribute.name)),
                ("type", text(&attribute_type(attribute.attribute_type))),
            ])
        })
        .collect();
    object([
        ("roles", roles(&context.roles)),
        ("attributes", Json::Array(attributes)),
    ])
}

/// The type of an attribute as SCY-2.2 writes it.
fn attribute_type(attribute_type: AttributeType) -> String {
    let scalar = |scalar: ScalarType| match scalar {
        ScalarType::Text => "string".to_owned(),
        ScalarType::Integer => "integer".to_owned(),
        ScalarType::Decimal { precision, scale } => format!("decimal({precision},{scale})"),
        ScalarType::Date => "date".to_owned(),
        ScalarType::Boolean => "boolean".to_owned(),
    };
    match attribute_type {
        AttributeType::Scalar(item) => scalar(item),
        AttributeType::List(item) => format!("list<{}>", scalar(item)),
    }
}
