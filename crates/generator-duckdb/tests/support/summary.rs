//! A model with the base dataset `ORDERS` and its summary `ORDERS_BY_REGION` (SUM-2), each
//! over its own relation, for the summary scans of E6-4.15.

use menura_contracts::ContractKind;
use menura_contracts::Sha256Hash;
use menura_contracts::types::catalog::{
    CatalogSchemaContract, CatalogSchemaDocument, Column, Engine, RelationSchema,
};
use menura_contracts::types::common::{LogicalType, Token};
use menura_generator_duckdb::{GenerateError, GeneratedSql};
use menura_model::internal_model::{
    Dataset, Descriptions, Field, InternalModel, ModelParts, NormalizedName, RelationshipGraph,
    TypedValue, ValueType,
};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::PlanNode;

use super::plan::{column, scan_of};
use super::stitch::huge_integer;
use super::{big_integer, decimal, location, parse, security};

/// One dataset: name, `source`, whether it is a summary, and its fields as (name, catalog
/// column, type).
struct DatasetShape {
    name: &'static str,
    source: &'static str,
    summary: bool,
    fields: Vec<(&'static str, &'static str, LogicalType)>,
}

fn shapes() -> Vec<DatasetShape> {
    vec![
        DatasetShape {
            name: "ORDERS",
            source: "main.orders",
            summary: false,
            fields: vec![
                ("REGION", "region", LogicalType::Text),
                ("AMOUNT", "amount", decimal(10, 2)),
                ("QUANTITY", "quantity", big_integer()),
            ],
        },
        DatasetShape {
            name: "ORDERS_BY_REGION",
            source: "main.orders_by_region",
            summary: true,
            fields: vec![
                ("REGION", "region", LogicalType::Text),
                ("AMOUNT_SUM", "amount_sum", decimal(38, 2)),
                ("AMOUNT_COUNT", "amount_count", big_integer()),
                ("AMOUNT_MAX", "amount_max", decimal(10, 2)),
                ("QUANTITY_SUM", "quantity_sum", huge_integer()),
                ("QUANTITY_COUNT", "quantity_count", big_integer()),
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
        summary: shape.summary,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location("/semantic_model/0/datasets/0"),
    }
}

/// The model of `ORDERS` and its summary `ORDERS_BY_REGION`.
pub fn summary_model() -> InternalModel {
    let parts = ModelParts {
        name: NormalizedName::new("SALES"),
        descriptions: Descriptions::default(),
        datasets: shapes().iter().map(dataset).collect(),
        relationships: Vec::new(),
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

/// The catalog schema of the two datasets: one column per field.
fn summary_catalog() -> CatalogSchemaDocument {
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
            name: "duckdb".to_owned(),
            version: "1.2.0".to_owned(),
        },
        relations,
    }
}

/// The `Scan` of `dataset` of the summary model that projects `fields`.
pub fn summary_scan(dataset: &str, fields: &[&str]) -> PlanNode {
    let columns = fields
        .iter()
        .map(|name| {
            let logical_type = shapes()
                .into_iter()
                .find(|shape| shape.name == dataset)
                .and_then(|shape| {
                    shape
                        .fields
                        .into_iter()
                        .find(|(field, _, _)| field == name)
                })
                .map(|(_, _, logical_type)| logical_type)
                .expect("a field of the summary model");
            column(name, logical_type, true)
        })
        .collect();
    scan_of(dataset, columns)
}

/// The SQL of `plan` over the summary model, for `query`.
pub fn generate_summary(plan: &PlanNode, query: &BoundQuery) -> Result<GeneratedSql, GenerateError> {
    menura_generator_duckdb::generate_sql(
        plan,
        &summary_catalog(),
        &summary_model(),
        query,
        &security(),
    )
}
