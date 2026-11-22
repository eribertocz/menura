//! The model, bound queries, branches and security contexts shared by the E4 tests.

#![allow(
    dead_code,
    reason = "each test file uses a different part of the shared support module"
)]

pub mod derived;
pub mod join_plan;
pub mod joins;
pub mod plan;
pub mod propagation;
pub mod stitch;

use menura_contracts::ContractKind;
use menura_contracts::Sha256Hash;
use menura_contracts::types::catalog::{
    CatalogSchemaContract, CatalogSchemaDocument, Column, Engine, RelationSchema,
};
use menura_contracts::types::common::Token;
use menura_contracts::types::common::{Grain, LogicalType, TimestampPrecision};
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_e4_plan::PlanError;
use menura_expressions::parse;
use menura_expressions::typing::{big_integer, decimal};
use menura_model::expression::{DateValue, Expression};
use menura_model::internal_model::{
    Additivity, AttributeType, Component, Dataset, DecomposabilityClass, Descriptions, Field,
    FieldPath, InternalModel, Metric, MetricClassification, ModelParts, NormalizedName,
    RelationshipGraph, RoleName, RowPolicy, ScalarType, SecurityAttribute, SecurityContext,
    SourceLocation, TypedValue, ValueType,
};
use menura_representations::bound_query::{
    BoundDateRange, BoundDimension, BoundDimensionKind, BoundFilter, BoundMetric, BoundMetricKind,
    BoundOrderItem, BoundQuery, BoundQueryParts, MetricPath,
};
use menura_representations::columns::Columns;
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::OutputName;
use menura_representations::origin::QueryLocation;
use menura_representations::paths::{Branch, BranchKind, Branches, ComponentKey, JoinTree};
use menura_representations::query_tree::QueryLimit;
use menura_representations::time::{QueryTime, TimeZoneName};
use menura_security::{AdmittedSecurityContext, admit_security_context};

/// The pointer of the `ORDERS` dataset in the model document.
pub const DATASET_POINTER: &str = "/semantic_model/0/datasets/0";

/// The pointer of the security extension entry of `ORDERS`.
pub const POLICY_POINTER: &str = "/semantic_model/0/datasets/0/custom_extensions/0/data";

/// A normalized name.
pub fn name(text: &str) -> NormalizedName {
    NormalizedName::new(text)
}

/// A location in the model document.
pub fn located(pointer: &str) -> SourceLocation {
    SourceLocation {
        pointer: pointer.to_owned(),
        line: 1,
        column: 1,
    }
}

/// A location in the query document.
pub fn at(pointer: &str) -> QueryLocation {
    QueryLocation::new(pointer, None)
}

/// The parsed tree of `text`.
pub fn tree(text: &str) -> Expression {
    parse(text).expect("an expression of the subset")
}

fn timestamp_with_time_zone() -> LogicalType {
    LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone: true,
    }
}

/// The fields of `ORDERS`, ordered by name: name, logical type, nullable.
pub fn order_fields() -> Vec<(&'static str, LogicalType, bool)> {
    vec![
        ("AMOUNT", decimal(12, 2), true),
        ("ORDERED_AT", timestamp_with_time_zone(), false),
        ("ORDER_DATE", LogicalType::Date, false),
        ("REGION", LogicalType::Text, true),
        ("STATUS", LogicalType::Text, false),
        ("TEAM", LogicalType::Text, true),
    ]
}

/// A field of `ORDERS` that reads the source column of its lower-case name.
pub fn field(name_text: &str, logical_type: LogicalType, nullable: bool) -> Field {
    Field {
        name: name(name_text),
        expression: Some(tree(&name_text.to_lowercase())),
        typed: Some(TypedValue {
            value_type: ValueType::Logical(logical_type),
            nullable,
        }),
        is_time: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: located(DATASET_POINTER),
    }
}

/// The dataset `ORDERS` with `policies`.
pub fn orders(policies: Vec<RowPolicy>) -> Dataset {
    Dataset {
        name: name("ORDERS"),
        source: "main.orders".to_owned(),
        fields: order_fields()
            .into_iter()
            .map(|(field_name, logical_type, nullable)| field(field_name, logical_type, nullable))
            .collect(),
        metrics: Vec::new(),
        primary_key: Vec::new(),
        unique_keys: Vec::new(),
        access: None,
        row_policies: policies,
        populations: Vec::new(),
        summary: false,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: located(DATASET_POINTER),
    }
}

/// A row policy of `ORDERS`.
pub fn policy(
    policy_name: &str,
    condition: &str,
    roles: Option<&[&str]>,
    exempt_roles: Option<&[&str]>,
) -> RowPolicy {
    let role_names = |names: &[&str]| names.iter().map(|role| RoleName::new(role)).collect();
    RowPolicy {
        name: policy_name.to_owned(),
        condition: Some(tree(condition)),
        roles: roles.map(role_names),
        exempt_roles: exempt_roles.map(role_names),
        location: located(POLICY_POINTER),
    }
}

/// A metric of the semantic model whose expression is `expression`, typed `logical_type`, with
/// one component per aggregate, in written order.
pub fn metric(metric_name: &str, expression: &str, logical_type: LogicalType) -> Metric {
    let expression = tree(expression);
    let mut components = Vec::new();
    collect_components(&expression, &mut components);
    Metric {
        name: name(metric_name),
        expression: Some(expression),
        typed: Some(TypedValue {
            value_type: ValueType::Logical(logical_type),
            nullable: true,
        }),
        classification: Some(MetricClassification {
            components,
            inputs: Vec::new(),
            home_datasets: vec![name("ORDERS")],
            additivity: Additivity::Additive,
            recomposable: true,
        }),
        declared_additivity: None,
        derived: None,
        context: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: located(&format!("/semantic_model/0/metrics/{metric_name}")),
    }
}

fn collect_components(expression: &Expression, components: &mut Vec<Component>) {
    match expression {
        Expression::Aggregate { call, filter } => components.push(Component {
            call: call.clone(),
            filter: filter.as_deref().cloned(),
            home_dataset: name("ORDERS"),
            class: DecomposabilityClass::Distributive,
        }),
        Expression::Binary { left, right, .. } => {
            collect_components(left, components);
            collect_components(right, components);
        }
        _ => {}
    }
}

/// The metrics of the semantic model, ordered by name.
pub fn metrics() -> Vec<Metric> {
    vec![
        metric(
            "AVERAGE_AMOUNT",
            "SUM(orders.amount) / COUNT(*)",
            decimal(38, 6),
        ),
        metric("ORDER_COUNT", "COUNT(*)", big_integer()),
        metric(
            "PAID_AMOUNT",
            "SUM(orders.amount) FILTER (WHERE orders.status = 'paid')",
            decimal(38, 2),
        ),
        metric("TOTAL_AMOUNT", "SUM(orders.amount)", decimal(38, 2)),
    ]
}

/// The declarations of the security context.
pub fn declarations() -> SecurityContext {
    let attribute = |attribute_name: &str, attribute_type| SecurityAttribute {
        name: attribute_name.to_owned(),
        attribute_type,
    };
    SecurityContext {
        roles: ["admin", "analyst", "auditor", "manager"]
            .into_iter()
            .map(RoleName::new)
            .collect(),
        attributes: vec![
            attribute("region", AttributeType::Scalar(ScalarType::Text)),
            attribute("regions", AttributeType::List(ScalarType::Text)),
            attribute("team", AttributeType::Scalar(ScalarType::Text)),
        ],
    }
}

/// The model with `ORDERS` (with `policies`) and the metrics of [`metrics`].
pub fn model(policies: Vec<RowPolicy>) -> InternalModel {
    model_with(vec![orders(policies)], metrics())
}

/// The model with `datasets` and the semantic-model `metrics`.
pub fn model_with(datasets: Vec<Dataset>, metrics: Vec<Metric>) -> InternalModel {
    let parts = ModelParts {
        name: name("SALES"),
        descriptions: Descriptions::default(),
        datasets,
        relationships: Vec::new(),
        graph: RelationshipGraph::default(),
        metrics,
        security_context: Some(declarations()),
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    let hash = Sha256Hash::try_from("0".repeat(64)).expect("a valid hash");
    InternalModel::new(parts, hash)
}

/// The admitted security context with `roles` and `attributes` (JSON texts).
pub fn user(roles: &str, attributes: &str) -> AdmittedSecurityContext {
    let text = format!(
        r#"{{"contract": "menura.security_context", "contract_version": "1.1.0",
            "subject": "user-1", "roles": {roles}, "attributes": {attributes}}}"#
    );
    admit_security_context(&text, Some(&declarations())).expect("an admissible context")
}

/// A user without roles or attributes.
pub fn anyone() -> AdmittedSecurityContext {
    user("[]", "{}")
}

/// The dimension `ORDERS.<field>` at `/query/dimensions/<index>`, with an optional grain.
pub fn dimension(field: &str, grain: Option<Grain>, index: usize) -> BoundDimension {
    BoundDimension {
        kind: BoundDimensionKind::Field {
            field: field_path(field),
            time_grain: grain,
        },
        output_name: OutputName::new(&field.to_lowercase()),
        origin: at(&format!("/query/dimensions/{index}")),
    }
}

/// The field `ORDERS.<field>`.
pub fn field_path(field: &str) -> FieldPath {
    FieldPath {
        dataset: name("ORDERS"),
        field: name(field),
    }
}

/// The metric `metric_name` of the semantic model at `/query/metrics/<index>`.
pub fn metric_reference(metric_name: &str, index: usize) -> BoundMetric {
    BoundMetric {
        kind: BoundMetricKind::Reference(MetricPath::SemanticModel(name(metric_name))),
        output_name: OutputName::new(&metric_name.to_lowercase()),
        origin: at(&format!("/query/metrics/{index}")),
    }
}

/// The filter `text` on `ORDERS` at `/query/filters/<index>`.
pub fn filter(text: &str, index: usize) -> BoundFilter {
    BoundFilter {
        expression: tree(text),
        datasets: vec![name("ORDERS")],
        origin: at(&format!("/query/filters/{index}")),
    }
}

/// The date `year-month-day`.
pub fn date(year: u16, month: u8, day: u8) -> DateValue {
    DateValue::new(year, month, day).expect("a valid date")
}

/// The date range `from`..=`to` on `ORDERS.<field>`.
pub fn date_range(field: &str, from: DateValue, to: DateValue) -> BoundDateRange {
    BoundDateRange {
        field: field_path(field),
        from,
        to,
        origin: at("/query/date_range"),
    }
}

/// The ordering term on output column `column`.
pub fn order(
    column: usize,
    direction: Direction,
    nulls: NullsOrder,
    index: usize,
) -> BoundOrderItem {
    BoundOrderItem {
        column,
        direction,
        nulls,
        origin: at(&format!("/query/order_by/{index}")),
    }
}

/// The row limit `rows`.
pub fn limit(rows: u64) -> QueryLimit {
    QueryLimit {
        rows,
        origin: at("/query/limit"),
    }
}

/// The wide-table query with `dimensions`, `metrics` and `parts`, in time zone `Europe/Madrid`.
pub fn query(
    dimensions: Vec<BoundDimension>,
    metrics: Vec<BoundMetric>,
    parts: BoundQueryParts,
) -> BoundQuery {
    let time = QueryTime {
        time_zone: TimeZoneName::new("Europe/Madrid"),
        reference_time: None,
        reference_date: None,
    };
    BoundQuery::new(
        Columns::dimensions_then_metrics(dimensions, metrics),
        time,
        parts,
    )
}

/// The single branch on `ORDERS` of a query whose metrics have `component_counts` components.
pub fn branches(component_counts: &[usize]) -> Branches {
    if component_counts.is_empty() {
        return Branches::dimension_only(name("ORDERS"), JoinTree::default());
    }
    let components = component_counts
        .iter()
        .enumerate()
        .flat_map(|(metric, count)| {
            (0..*count).map(move |component| ComponentKey { metric, component })
        })
        .collect();
    Branches::new(vec![Branch {
        home: name("ORDERS"),
        kind: BranchKind::Plain,
        components,
        join_tree: JoinTree::default(),
    }])
    .expect("one branch")
}

/// The catalog columns of `main.orders` that only dataset metrics read: name as the catalog
/// spells it, logical type, nullable.
pub fn source_columns() -> Vec<(&'static str, LogicalType, bool)> {
    vec![
        ("amount", decimal_amount(), true),
        ("amount_cents", big_integer(), false),
        ("Amount Cents", big_integer(), true),
    ]
}

/// The type of the source column `amount`.
pub fn decimal_amount() -> LogicalType {
    decimal(12, 2)
}

/// The admitted catalog schema: the relation `main.orders` of `ORDERS`, with the columns of
/// [`source_columns`].
pub fn catalog() -> CatalogSchemaDocument {
    let columns = source_columns()
        .into_iter()
        .map(|(column_name, logical_type, nullable)| Column {
            name: column_name.to_owned(),
            logical_type,
            physical_type: "unused".to_owned(),
            nullable,
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
        relations: vec![RelationSchema {
            source: "main.orders".to_owned(),
            columns,
        }],
    }
}

/// The plan of `query` over `model` and [`catalog`].
pub fn build_plan(
    branches: &Branches,
    query: &BoundQuery,
    security: &AdmittedSecurityContext,
    model: &InternalModel,
) -> Result<PlanNode, PlanError> {
    menura_e4_plan::build_plan(branches, query, security, model, &catalog())
}
