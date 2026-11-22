//! A model for the `Population` node (E4-3.12, POP-3): `CUSTOMERS` with validity intervals,
//! `STATEMENTS` with one row per customer and month, and `MONTHS`, the query's unit of
//! periods.

use menura_contracts::ContractKind;
use menura_contracts::Sha256Hash;
use menura_contracts::types::catalog::{
    CatalogSchemaContract, CatalogSchemaDocument, Column, Engine, RelationSchema,
};
use menura_contracts::types::common::{Grain, LogicalType, TimestampPrecision, Token};
use menura_generator_duckdb::{GenerateError, GeneratedSql};
use menura_model::internal_model::{
    Dataset, Descriptions, Field, InternalModel, ModelParts, NormalizedName, PartialPeriod,
    RelationshipGraph, TypedValue, ValueType,
};
use menura_representations::bound_query::BoundQuery;
use menura_representations::logical_plan::{
    Membership, Operation, OutputSchema, PlanNode, PopulationRule, SchemaColumn,
};
use menura_representations::names::ColumnName;

use super::{big_integer, decimal, from_model, instant, location, parse, query, security};

/// One dataset: name, `source`, and its fields as (name, catalog column, type).
type Shape = (
    &'static str,
    &'static str,
    Vec<(&'static str, &'static str, LogicalType)>,
);

/// A timestamp without time zone.
fn local_timestamp() -> LogicalType {
    LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone: false,
    }
}

/// The datasets, sorted by name as the model keeps them.
fn shapes() -> Vec<Shape> {
    let mut shapes = balance_shapes();
    shapes.extend(other_shapes());
    shapes
}

/// The datasets of balances, in name order.
fn balance_shapes() -> Vec<Shape> {
    vec![
        (
            "ADDED_BALANCES",
            "main.added_balances",
            vec![
                ("CUSTOMER_ID", "customer_id", big_integer()),
                ("BALANCE_DATE", "balance_date", LogicalType::Date),
                ("BRANCH", "branch", LogicalType::Text),
                ("BALANCE", "balance", decimal(12, 2)),
                ("PERIOD", "period", LogicalType::Date),
            ],
        ),
        (
            "BALANCES",
            "main.balances",
            vec![
                ("CUSTOMER_ID", "customer_id", big_integer()),
                ("BALANCE_DATE", "balance_date", LogicalType::Date),
                ("BRANCH", "branch", LogicalType::Text),
                ("BALANCE", "balance", decimal(12, 2)),
            ],
        ),
    ]
}

/// The other datasets, in name order, after the balances.
fn other_shapes() -> Vec<Shape> {
    vec![
        (
            "CUSTOMERS",
            "main.customers",
            vec![
                ("CUSTOMER_ID", "customer_id", big_integer()),
                ("VALID_FROM", "valid_from", LogicalType::Date),
                ("VALID_TO", "valid_to", LogicalType::Date),
                ("IS_BILLABLE", "is_billable", LogicalType::Boolean),
                ("ACTIVE_FROM", "active_from", instant()),
                ("ACTIVE_TO", "active_to", local_timestamp()),
            ],
        ),
        (
            "MONTHS",
            "main.months",
            vec![("PERIOD", "period", LogicalType::Date)],
        ),
        (
            "STATEMENTS",
            "main.statements",
            vec![
                ("CUSTOMER_ID", "customer_id", big_integer()),
                ("STATEMENT_MONTH", "statement_month", LogicalType::Date),
            ],
        ),
        (
            "STOCK",
            "main.stock",
            vec![
                ("WAREHOUSE", "warehouse", LogicalType::Text),
                ("PRODUCT_ID", "product_id", big_integer()),
                ("CATEGORY", "category", LogicalType::Text),
                ("QUANTITY", "quantity", big_integer()),
            ],
        ),
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

fn dataset((name, source, fields): &Shape) -> Dataset {
    Dataset {
        name: NormalizedName::new(name),
        source: (*source).to_owned(),
        fields: fields
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

fn population_model() -> InternalModel {
    let parts = ModelParts {
        name: NormalizedName::new("BILLING"),
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

fn population_catalog() -> CatalogSchemaDocument {
    let relations = shapes()
        .into_iter()
        .map(|(_, source, fields)| RelationSchema {
            source: source.to_owned(),
            columns: fields
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

fn plan_column(name: &str, logical_type: LogicalType) -> SchemaColumn {
    SchemaColumn {
        name: ColumnName::new(name),
        logical_type,
        nullable: true,
    }
}

/// The `Scan` of `dataset` that projects `fields`.
pub fn population_scan(dataset: &str, fields: &[&str]) -> PlanNode {
    let (_, _, shape) = shapes()
        .into_iter()
        .find(|(name, _, _)| *name == dataset)
        .expect("a dataset of the population model");
    let columns = fields
        .iter()
        .map(|name| {
            let (_, _, logical_type) = shape
                .iter()
                .find(|(field, _, _)| field == name)
                .expect("a field of the dataset");
            plan_column(name, logical_type.clone())
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

/// The validity membership of `CUSTOMERS` under `partial_period`.
pub fn validity(partial_period: PartialPeriod) -> Membership {
    Membership::Validity {
        valid_from: ColumnName::new("VALID_FROM"),
        valid_to: ColumnName::new("VALID_TO"),
        partial_period,
    }
}

/// The `Population` of `CUSTOMER_ID` over the rows `input` of `dataset`, by `membership`
/// and `required_when`, at `grain`, over the periods of `MONTHS`.
pub fn population(
    input: PlanNode,
    dataset: &str,
    membership: Membership,
    required_when: Option<&str>,
    grain: Grain,
) -> PlanNode {
    PlanNode::new(
        Operation::Population {
            input: Box::new(input),
            dataset: NormalizedName::new(dataset),
            population: NormalizedName::new("EXPECTED_CUSTOMERS"),
            grain,
            periods: Box::new(population_scan("MONTHS", &["PERIOD"])),
            period: ColumnName::new("PERIOD"),
            rule: PopulationRule {
                entity: vec![ColumnName::new("CUSTOMER_ID")],
                membership,
                required_when: required_when.map(parse),
            },
        },
        OutputSchema::new(vec![
            SchemaColumn {
                name: ColumnName::new("CUSTOMER_ID"),
                logical_type: big_integer(),
                nullable: true,
            },
            SchemaColumn {
                name: ColumnName::new("PERIOD"),
                logical_type: LogicalType::Date,
                nullable: false,
            },
        ]),
        from_model("/semantic_model/0/datasets/0/custom_extensions/0"),
    )
}

/// The SQL of `plan` over the population model, for a query of `CUSTOMER_ID` and `PERIOD`.
pub fn generate_population(plan: &PlanNode) -> Result<GeneratedSql, GenerateError> {
    generate_billing(plan, &query(&["CUSTOMER_ID", "PERIOD"], &[]))
}

/// The SQL of `plan` over the population model, for `query`.
pub fn generate_billing(
    plan: &PlanNode,
    query: &BoundQuery,
) -> Result<GeneratedSql, GenerateError> {
    menura_generator_duckdb::generate_sql(
        plan,
        &population_catalog(),
        &population_model(),
        query,
        &security(),
    )
}
