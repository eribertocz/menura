//! A model with three datasets joined by many-to-one relationships, for the `SemiJoin` of
//! policy propagation (E4-3.4, E4-6.5a): `ORDERS` to `CUSTOMERS` by `ORDERS_CUSTOMER`, and
//! `CUSTOMERS` to `COUNTRIES` by `CUSTOMERS_COUNTRY`.

use menura_contracts::ContractKind;
use menura_contracts::Sha256Hash;
use menura_contracts::types::catalog::{
    CatalogSchemaContract, CatalogSchemaDocument, Column, Engine, RelationSchema,
};
use menura_contracts::types::common::{LogicalType, Token};
use menura_generator_clickhouse::{GenerateError, GeneratedSql};
use menura_model::internal_model::{
    Dataset, Descriptions, Field, InternalModel, ModelParts, NormalizedName, Relationship,
    RelationshipGraph, TypedValue, ValueType,
};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{
    JoinCondition, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::ColumnName;

use super::{big_integer, from_model, location, parse, security};

/// One dataset: name, `source`, and its fields as (name, catalog column, type).
struct DatasetShape {
    name: &'static str,
    source: &'static str,
    fields: Vec<(&'static str, &'static str, LogicalType)>,
}

fn shapes() -> Vec<DatasetShape> {
    vec![
        DatasetShape {
            name: "COUNTRIES",
            source: "main.countries",
            fields: vec![
                ("COUNTRY_ID", "country_id", big_integer()),
                ("IS_OPEN", "is_open", LogicalType::Boolean),
            ],
        },
        DatasetShape {
            name: "CUSTOMERS",
            source: "main.customers",
            fields: vec![
                ("CUSTOMER_ID", "customer_id", big_integer()),
                ("COUNTRY_ID", "country_id", big_integer()),
                ("REGION", "Sales Region", LogicalType::Text),
            ],
        },
        DatasetShape {
            name: "ORDERS",
            source: "main.orders",
            fields: vec![
                ("ORDER_ID", "order_id", big_integer()),
                ("CUSTOMER_ID", "customer_id", big_integer()),
            ],
        },
    ]
}

fn field(name: &str, column: &str, logical_type: LogicalType) -> Field {
    Field {
        name: NormalizedName::new(name),
        expression: Some(parse(&format!("\"{column}\""))),
        typed: Some(TypedValue {
            value_type: ValueType::Logical(logical_type),
            nullable: true,
        }),
        is_time: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location("/semantic_model/0/datasets/0/fields/0"),
    }
}

fn dataset(shape: &DatasetShape) -> Dataset {
    Dataset {
        name: NormalizedName::new(shape.name),
        source: shape.source.to_owned(),
        fields: shape
            .fields
            .iter()
            .map(|(name, column, logical_type)| field(name, column, logical_type.clone()))
            .collect(),
        metrics: Vec::new(),
        primary_key: Vec::new(),
        unique_keys: Vec::new(),
        access: None,
        row_policies: Vec::new(),
        populations: Vec::new(),
        summary: false,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location("/semantic_model/0/datasets/0"),
    }
}

fn relationship(name: &str, from: &str, to: &str, column: &str) -> Relationship {
    Relationship {
        name: NormalizedName::new(name),
        from: NormalizedName::new(from),
        to: NormalizedName::new(to),
        from_columns: vec![NormalizedName::new(column)],
        to_columns: vec![NormalizedName::new(column)],
        temporal: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location("/semantic_model/0/relationships/0"),
    }
}

/// The model of the three datasets and their two relationships.
pub fn propagation_model() -> InternalModel {
    let parts = ModelParts {
        name: NormalizedName::new("SALES"),
        descriptions: Descriptions::default(),
        datasets: shapes().iter().map(dataset).collect(),
        relationships: vec![
            relationship("CUSTOMERS_COUNTRY", "CUSTOMERS", "COUNTRIES", "COUNTRY_ID"),
            relationship("ORDERS_CUSTOMER", "ORDERS", "CUSTOMERS", "CUSTOMER_ID"),
        ],
        graph: RelationshipGraph::default(),
        metrics: Vec::new(),
        security_context: None,
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    let hash = Sha256Hash::try_from("0".repeat(64)).expect("a hash");
    InternalModel::new(parts, hash)
}

/// The catalog schema of the three datasets: one column per field.
fn propagation_catalog() -> CatalogSchemaDocument {
    let relations = shapes()
        .into_iter()
        .map(|shape| RelationSchema {
            source: shape.source.to_owned(),
            columns: shape
                .fields
                .into_iter()
                .map(|(_, column, logical_type)| Column {
                    name: column.to_owned(),
                    logical_type,
                    physical_type: "unused".to_owned(),
                    nullable: true,
                })
                .collect(),
        })
        .collect();
    CatalogSchemaDocument {
        contract: CatalogSchemaContract::CatalogSchema,
        contract_version: ContractKind::CatalogSchema.latest_version(),
        document_id: Token {
            origin: "tests".to_owned(),
            snapshot: "1".to_owned(),
        },
        engine: Engine {
            name: "clickhouse".to_owned(),
            version: "24.8.0".to_owned(),
        },
        relations,
    }
}

/// The type of the field `name` of the dataset `dataset`.
fn field_type(dataset: &str, name: &str) -> LogicalType {
    shapes()
        .into_iter()
        .find(|shape| shape.name == dataset)
        .and_then(|shape| {
            shape
                .fields
                .into_iter()
                .find(|(field, _, _)| *field == name)
        })
        .map(|(_, _, logical_type)| logical_type)
        .expect("a field of the propagation model")
}

/// The `Scan` of `dataset` that projects `fields`.
pub fn scan_fields(dataset: &str, fields: &[&str]) -> PlanNode {
    let columns = fields
        .iter()
        .map(|name| (*name, field_type(dataset, name)))
        .collect::<Vec<_>>();
    scan_typed(dataset, &columns)
}

/// The `Scan` of `dataset` that projects `columns`, each a field with the plan type given.
pub fn scan_typed(dataset: &str, columns: &[(&str, LogicalType)]) -> PlanNode {
    let columns = columns
        .iter()
        .map(|(name, logical_type)| SchemaColumn {
            name: ColumnName::new(name),
            logical_type: logical_type.clone(),
            nullable: true,
        })
        .collect();
    PlanNode::new(
        Operation::Scan {
            dataset: NormalizedName::new(dataset),
            row_identity: None,
        },
        OutputSchema::new(columns),
        from_model("/semantic_model/0/datasets/0"),
    )
}

/// The `Scan` of `ORDERS` that projects `ORDER_ID` and `CUSTOMER_ID`.
pub fn orders_scan() -> PlanNode {
    scan_fields("ORDERS", &["ORDER_ID", "CUSTOMER_ID"])
}

/// The `Filter` of `input` by the row policy `predicate`.
pub fn policy(input: PlanNode, predicate: &str) -> PlanNode {
    super::filter(
        input,
        predicate,
        from_model("/semantic_model/0/datasets/0/row_policies/0"),
    )
}

/// The `SemiJoin` of `input` along `relationship`, whose key is `key` on both sides, against
/// the rows `related_all` and the visible rows `related_visible`, with the schema of `input`.
pub fn semi_join(
    input: PlanNode,
    relationship: &str,
    key: &str,
    related_all: PlanNode,
    related_visible: PlanNode,
) -> PlanNode {
    let on = vec![JoinCondition {
        left: ColumnName::new(key),
        right: ColumnName::new(key),
    }];
    semi_join_on(input, relationship, on, related_all, related_visible)
}

/// The `SemiJoin` of `input` along `relationship` by the key equalities `on`, against the rows
/// `related_all` and the visible rows `related_visible`, with the schema of `input`.
pub fn semi_join_on(
    input: PlanNode,
    relationship: &str,
    on: Vec<JoinCondition>,
    related_all: PlanNode,
    related_visible: PlanNode,
) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::SemiJoin {
            input: Box::new(input),
            relationship: NormalizedName::new(relationship),
            on,
            temporal: None,
            related_all: Box::new(related_all),
            related_visible: Box::new(related_visible),
        },
        schema,
        from_model("/semantic_model/0/relationships/0"),
    )
}

/// The SQL of `plan` over the propagation model, for `query`.
pub fn generate_propagation(
    plan: &PlanNode,
    query: &BoundQuery,
) -> Result<GeneratedSql, GenerateError> {
    menura_generator_clickhouse::generate_sql(
        plan,
        &propagation_catalog(),
        &propagation_model(),
        query,
        &security(),
    )
}
