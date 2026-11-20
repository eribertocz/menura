//! Hand-built inputs of E6: a model with one dataset, bound queries, security contexts and
//! plan nodes that follow the plan conventions of issue #251.

#![allow(
    dead_code,
    unused_imports,
    reason = "each test file uses a different part of the shared support module"
)]

use menura_contracts::ContractKind;
use menura_contracts::Sha256Hash;
use menura_contracts::types::catalog::{
    CatalogSchemaContract, CatalogSchemaDocument, Column, Engine, RelationSchema,
};
use menura_contracts::types::common::Token;
use menura_contracts::types::common::{IntegerBits, LogicalType, TimestampPrecision};
use menura_generator_duckdb::{GenerateError, GeneratedSql};

mod plan;

use menura_model::expression::Expression;
use menura_model::internal_model::{
    AttributeType, Dataset, Descriptions, Field, FieldPath, InternalModel, ModelParts,
    NormalizedName, RelationshipGraph, ScalarType, SecurityAttribute, SecurityContext,
    SourceLocation, TypedValue, ValueType,
};
use menura_representations::bound_query::{
    BoundDimension, BoundDimensionKind, BoundMetric, BoundMetricKind, BoundQuery, BoundQueryParts,
    MetricPath,
};
use menura_representations::columns::Columns;
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::OutputName;
use menura_representations::origin::{Origin, QueryLocation};
use menura_representations::time::{QueryTime, TimeZoneName};
use menura_security::{AdmittedSecurityContext, admit_security_context};
pub use plan::{
    aggregate, column, component, component_of, filter, limit, named, project, region_count, scan,
    scan_of, sort,
};

/// The 64-bit signed integer.
pub fn big_integer() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::SixtyFour,
        signed: true,
    }
}

/// `decimal(precision, scale)`.
pub fn decimal(precision: u8, scale: u8) -> LogicalType {
    LogicalType::Decimal { precision, scale }
}

/// `timestamp with time zone`, precision `us`.
pub fn instant() -> LogicalType {
    LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone: true,
    }
}

/// The fields of `ORDERS`: name, expression over the catalog columns, type.
fn order_fields() -> Vec<(&'static str, &'static str, LogicalType)> {
    vec![
        ("AMOUNT", "amount", decimal(10, 2)),
        (
            "CREATED_MONTH",
            "DATE_TRUNC('month', created_at)",
            instant(),
        ),
        ("CREATED_AT", "created_at", instant()),
        ("IS_PAID", "status = 'paid'", LogicalType::Boolean),
        (
            "LINE_SQUARE",
            "quantity * quantity",
            LogicalType::Integer {
                bits: IntegerBits::OneHundredTwentyEight,
                signed: true,
            },
        ),
        ("ORDERED_ON", "ordered_on", LogicalType::Date),
        ("QUANTITY", "quantity", big_integer()),
        ("REGION", "\"Sales Region\"", LogicalType::Text),
        // `units: quantity`, as E0 stores it: the delimited identifier of the normalized name.
        ("UNITS", "\"QUANTITY\"", big_integer()),
        (
            "SHIPPED_AT",
            "shipped_at",
            LogicalType::Timestamp {
                precision: TimestampPrecision::Us,
                with_time_zone: false,
            },
        ),
    ]
}

fn location(pointer: &str) -> SourceLocation {
    SourceLocation {
        pointer: pointer.to_owned(),
        line: 1,
        column: 1,
    }
}

fn field(name: &str, expression: &str, logical_type: LogicalType) -> Field {
    Field {
        name: NormalizedName::new(name),
        expression: Some(parse(expression)),
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

fn dataset(name: &str, source: &str) -> Dataset {
    let fields = order_fields()
        .into_iter()
        .map(|(name, expression, logical_type)| field(name, expression, logical_type))
        .collect();
    Dataset {
        name: NormalizedName::new(name),
        source: source.to_owned(),
        fields,
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

fn attribute(name: &str, attribute_type: AttributeType) -> SecurityAttribute {
    SecurityAttribute {
        name: name.to_owned(),
        attribute_type,
    }
}

/// The declared attributes: `max_amount decimal(10,2)`, `regions list<text>`,
/// `tenant integer`, `tenants list<integer>`.
fn declarations() -> SecurityContext {
    SecurityContext {
        roles: Vec::new(),
        attributes: vec![
            attribute(
                "max_amount",
                AttributeType::Scalar(ScalarType::Decimal {
                    precision: 10,
                    scale: 2,
                }),
            ),
            attribute("regions", AttributeType::List(ScalarType::Text)),
            attribute("tenant", AttributeType::Scalar(ScalarType::Integer)),
            attribute("tenants", AttributeType::List(ScalarType::Integer)),
        ],
    }
}

/// A model whose datasets are `ORDERS` over `source` and `STAGED` over a query.
pub fn model_with_source(source: &str) -> InternalModel {
    let parts = ModelParts {
        name: NormalizedName::new("SALES"),
        descriptions: Descriptions::default(),
        datasets: vec![
            dataset("ORDERS", source),
            dataset("STAGED", "  select * from \"raw\".\"orders\""),
        ],
        relationships: Vec::new(),
        graph: RelationshipGraph::default(),
        metrics: Vec::new(),
        security_context: Some(declarations()),
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    let hash = Sha256Hash::try_from("0".repeat(64)).expect("a hash");
    InternalModel::new(parts, hash)
}

/// The model whose `ORDERS` reads `main.orders`.
pub fn model() -> InternalModel {
    model_with_source("main.orders")
}

/// The security context with `attributes`, a JSON object text.
pub fn security_with(attributes: &str) -> AdmittedSecurityContext {
    let text = format!(
        r#"{{"contract": "menura.security_context", "contract_version": "1.1.0",
            "subject": "user-7f3a", "roles": [], "attributes": {attributes}}}"#
    );
    admit_security_context(&text, Some(&declarations())).expect("an admitted context")
}

/// The security context with every attribute set.
pub fn security() -> AdmittedSecurityContext {
    security_with(
        r#"{"max_amount": "100.50", "regions": ["north", "south"], "tenant": 7,
            "tenants": [1, 2]}"#,
    )
}

/// The expression `text`.
pub fn parse(text: &str) -> Expression {
    menura_expressions::parse(text).expect("an expression")
}

/// A location in the query document.
pub fn at(pointer: &str) -> QueryLocation {
    QueryLocation::new(pointer, None)
}

/// The origin of a query element.
pub fn from_query(pointer: &str) -> Origin {
    Origin::Query(at(pointer))
}

/// The origin of a model element.
pub fn from_model(pointer: &str) -> Origin {
    Origin::Model(location(pointer))
}

fn dimension(name: &str) -> BoundDimension {
    BoundDimension {
        kind: BoundDimensionKind::Field {
            field: FieldPath {
                dataset: NormalizedName::new("ORDERS"),
                field: NormalizedName::new(name),
            },
            time_grain: None,
        },
        output_name: OutputName::new(name),
        origin: at("/query/dimensions/0"),
    }
}

fn metric(name: &str) -> BoundMetric {
    BoundMetric {
        kind: BoundMetricKind::Reference(MetricPath::SemanticModel(NormalizedName::new(name))),
        output_name: OutputName::new(name),
        origin: at("/query/metrics/0"),
    }
}

/// The bound query with `dimensions` then `metrics`, in the time zone `time_zone`.
pub fn query_in(dimensions: &[&str], metrics: &[&str], time_zone: &str) -> BoundQuery {
    let columns = Columns::dimensions_then_metrics(
        dimensions.iter().map(|name| dimension(name)).collect(),
        metrics.iter().map(|name| metric(name)).collect(),
    );
    let time = QueryTime {
        time_zone: TimeZoneName::new(time_zone),
        reference_time: None,
        reference_date: None,
    };
    BoundQuery::new(columns, time, BoundQueryParts::default())
}

/// The bound query with `dimensions` then `metrics`, in `Europe/Madrid`.
pub fn query(dimensions: &[&str], metrics: &[&str]) -> BoundQuery {
    query_in(dimensions, metrics, "Europe/Madrid")
}

/// The catalog columns of every relation: name as the catalog spells it, type.
fn catalog_columns() -> Vec<Column> {
    let integer = big_integer();
    let timestamp = LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone: false,
    };
    [
        ("Amount", decimal(10, 2)),
        ("created_at", instant()),
        ("ordered_on", LogicalType::Date),
        ("quantity", integer),
        ("Sales Region", LogicalType::Text),
        ("shipped_at", timestamp),
        ("status", LogicalType::Text),
    ]
    .into_iter()
    .map(|(name, logical_type)| Column {
        name: name.to_owned(),
        logical_type,
        physical_type: "unused".to_owned(),
        nullable: true,
    })
    .collect()
}

/// The catalog schema with one relation, of the same columns, per dataset `source` of `model`.
pub fn catalog_for(model: &InternalModel) -> CatalogSchemaDocument {
    let relations = model
        .parts()
        .datasets
        .iter()
        .map(|dataset| RelationSchema {
            source: dataset.source.clone(),
            columns: catalog_columns(),
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

/// The SQL of `plan` over `model` and its catalog (`catalog_for`).
pub fn generate_sql(
    plan: &PlanNode,
    model: &InternalModel,
    query: &BoundQuery,
    security: &AdmittedSecurityContext,
) -> Result<GeneratedSql, GenerateError> {
    menura_generator_duckdb::generate_sql(plan, &catalog_for(model), model, query, security)
}

/// The SQL of `plan` for the dimension-only `query`, with every attribute set.
pub fn generate(plan: &PlanNode, query: &BoundQuery) -> Result<GeneratedSql, GenerateError> {
    generate_sql(plan, &model(), query, &security())
}

/// Asserts that `actual` is the golden file `expected`, printing the actual text otherwise.
pub fn assert_golden(actual: &str, expected: &str) {
    assert!(
        actual == expected,
        "the SQL differs from its golden file; actual SQL:\n{actual}"
    );
}
