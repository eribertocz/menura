//! The model, plans and queries of the summary tests: `ORDERS`, its summary `ORDERS_MONTHLY`
//! by region, month and customer, and `CUSTOMERS`.

mod eligibility;
mod expressions;
mod grain;
mod plans;
mod summary_branch;

pub(super) use plans::{
    aggregate, by_region_and_month, component, deduplicate, join, policy, query_filter, scan,
    semi_join, visible_orders,
};

use menura_contracts::Sha256Hash;
use menura_contracts::types::catalog::{
    CatalogSchemaContract, CatalogSchemaDocument, Column, Engine, RelationSchema,
};
use menura_contracts::types::common::{FloatBits, Grain, IntegerBits, LogicalType, Token};
use menura_contracts::{ContractKind, types::common::TimestampPrecision};
use menura_model::expression::{
    BinaryOperator, CastTarget, DateValue, Expression, Literal, SimpleAggregate,
};
use menura_model::internal_model::{
    Dataset, Descriptions, Field, FieldPath, InternalModel, ModelParts, NormalizedName,
    RelationshipGraph, SourceLocation, Summary, SummaryDimension, SummaryMeasure, TypedValue,
    ValueType,
};
use menura_representations::bound_query::AuthorizedSubgraph;
use menura_representations::bound_query::{
    BoundDateRange, BoundFilter, BoundQuery, BoundQueryParts,
};
use menura_representations::columns::Columns;
use menura_representations::origin::QueryLocation;
use menura_representations::time::{QueryTime, TimeZoneName};

use crate::context::RewriteContext;
use crate::evidence::Evidence;
use crate::test_support::anyone;
use expressions::{aggregate_of, cast, column, qualified};

/// The base dataset.
pub(super) const ORDERS: &str = "ORDERS";
/// The summary of `ORDERS`.
pub(super) const MONTHLY: &str = "ORDERS_MONTHLY";

pub(super) fn integer() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::ThirtyTwo,
        signed: true,
    }
}

pub(super) fn big_integer() -> LogicalType {
    LogicalType::Integer {
        bits: IntegerBits::SixtyFour,
        signed: true,
    }
}

fn zoned_timestamp() -> LogicalType {
    LogicalType::Timestamp {
        precision: TimestampPrecision::Us,
        with_time_zone: true,
    }
}

/// The fields of `dataset` as (name, expression, type): each reads the column of its name
/// in lower case, but `CREATED_DAY`, the date of `created_at`.
fn fields_of(dataset: &str) -> Vec<(&'static str, Expression, LogicalType)> {
    let decimal = |precision, scale| LogicalType::Decimal { precision, scale };
    let double = LogicalType::Float {
        bits: FloatBits::SixtyFour,
    };
    let fields = match dataset {
        ORDERS => vec![
            ("REGION", LogicalType::Text),
            ("ORDER_DATE", LogicalType::Date),
            ("CUSTOMER_ID", integer()),
            ("SALES_REP_ID", integer()),
            ("AMOUNT", decimal(10, 2)),
            ("QUANTITY", big_integer()),
            ("PRICE", double),
            ("SHIPPED_DATE", LogicalType::Date),
            ("CREATED_AT", zoned_timestamp()),
            ("CREATED_DAY", LogicalType::Date),
        ],
        MONTHLY => vec![
            ("REGION", LogicalType::Text),
            ("MONTH", LogicalType::Date),
            ("CUSTOMER_ID", integer()),
            ("AMOUNT_SUM", decimal(38, 2)),
            ("AMOUNT_COUNT", big_integer()),
            ("AMOUNT_MAX", decimal(10, 2)),
            ("PRICE_SUM", double),
        ],
        _ => vec![("CUSTOMER_ID", integer()), ("COUNTRY", LogicalType::Text)],
    };
    fields
        .into_iter()
        .map(|(name, logical_type)| {
            let expression = if name == "CREATED_DAY" {
                cast(column("created_at"), CastTarget::Date)
            } else {
                column(&name.to_lowercase())
            };
            (name, expression, logical_type)
        })
        .collect()
}

/// The fields of `dataset`, ordered by name as the internal model orders them.
fn sorted_fields(dataset: &str) -> Vec<(&'static str, Expression, LogicalType)> {
    let mut fields = fields_of(dataset);
    fields.sort_by(|left, right| left.0.cmp(right.0));
    fields
}

/// The type of the field `field` of `dataset`.
pub(super) fn type_of(dataset: &str, field: &str) -> LogicalType {
    fields_of(dataset)
        .into_iter()
        .find(|(name, _, _)| *name == field)
        .map(|(_, _, logical_type)| logical_type)
        .expect("a field of the fixture")
}

pub(super) fn location() -> SourceLocation {
    SourceLocation {
        pointer: "/semantic_model/0".to_owned(),
        line: 1,
        column: 1,
    }
}

fn dataset(name: &str) -> Dataset {
    Dataset {
        name: NormalizedName::new(name),
        source: format!("shop.main.{}", name.to_lowercase()),
        fields: sorted_fields(name)
            .into_iter()
            .map(|(field, expression, logical_type)| Field {
                name: NormalizedName::new(field),
                expression: Some(expression),
                typed: Some(TypedValue {
                    value_type: ValueType::Logical(logical_type),
                    nullable: true,
                }),
                is_time: None,
                descriptions: Descriptions::default(),
                query_errors: Vec::new(),
                location: location(),
            })
            .collect(),
        metrics: Vec::new(),
        primary_key: Vec::new(),
        unique_keys: Vec::new(),
        access: None,
        row_policies: Vec::new(),
        populations: Vec::new(),
        summary: name == MONTHLY,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

/// `ORDERS_MONTHLY`: by `REGION`, `ORDER_DATE` at month grain and `CUSTOMER_ID`, with the
/// sum, count and maximum of `AMOUNT` and the sum of `PRICE`.
pub(super) fn monthly() -> Summary {
    let dimension = |field: &str, base: &str, grain| SummaryDimension {
        field: NormalizedName::new(field),
        base: NormalizedName::new(base),
        grain,
    };
    let measure = |field: &str, function, argument: &str| SummaryMeasure {
        field: NormalizedName::new(field),
        base_aggregate: Some(aggregate_of(function, qualified("orders", argument))),
    };
    Summary {
        dataset: NormalizedName::new(MONTHLY),
        of: NormalizedName::new(ORDERS),
        dimensions: vec![
            dimension("REGION", "REGION", None),
            dimension("MONTH", "ORDER_DATE", Some(Grain::Month)),
            dimension("CUSTOMER_ID", "CUSTOMER_ID", None),
        ],
        measures: vec![
            measure("AMOUNT_SUM", SimpleAggregate::Sum, "amount"),
            measure("AMOUNT_COUNT", SimpleAggregate::Count, "amount"),
            measure("AMOUNT_MAX", SimpleAggregate::Maximum, "amount"),
            measure("PRICE_SUM", SimpleAggregate::Sum, "price"),
        ],
        coverage_field: NormalizedName::new("MONTH"),
        definition_hash: Sha256Hash::try_from("1".repeat(64)).expect("a hash"),
        location: location(),
    }
}

/// The model of `ORDERS`, `ORDERS_MONTHLY` and `CUSTOMERS`, changed by `change`.
pub(super) fn model_with(change: impl FnOnce(&mut ModelParts)) -> InternalModel {
    let mut parts = ModelParts {
        name: NormalizedName::new("SHOP"),
        descriptions: Descriptions::default(),
        datasets: vec![dataset("CUSTOMERS"), dataset(ORDERS), dataset(MONTHLY)],
        relationships: Vec::new(),
        graph: RelationshipGraph::default(),
        metrics: Vec::new(),
        security_context: None,
        summaries: vec![monthly()],
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    change(&mut parts);
    InternalModel::new(parts, Sha256Hash::try_from("0".repeat(64)).expect("a hash"))
}

pub(super) fn model() -> InternalModel {
    model_with(|_| ())
}

/// The catalog schema: one column per field that reads a column, `created_at` with time zone.
pub(super) fn schema() -> CatalogSchemaDocument {
    let relations = [ORDERS, MONTHLY, "CUSTOMERS"]
        .into_iter()
        .map(|name| RelationSchema {
            source: format!("shop.main.{}", name.to_lowercase()),
            columns: fields_of(name)
                .into_iter()
                .filter_map(|(_, expression, logical_type)| match expression {
                    Expression::Reference(reference) => Some(Column {
                        name: reference.name.text().to_owned(),
                        logical_type,
                        physical_type: "unused".to_owned(),
                        nullable: true,
                    }),
                    _ => None,
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

pub(super) fn at() -> QueryLocation {
    QueryLocation::new("/query", None)
}

/// A query in UTC with `filters` over `ORDERS` and an optional date range, by a user who has
/// access to `ORDERS` when `access` holds.
pub(super) fn query_of(
    filters: &[Expression],
    date_range: Option<(&str, DateValue, DateValue)>,
    access: bool,
) -> BoundQuery {
    let datasets = if access {
        vec![
            NormalizedName::new("CUSTOMERS"),
            NormalizedName::new(ORDERS),
        ]
    } else {
        Vec::new()
    };
    let parts = BoundQueryParts {
        filters: filters
            .iter()
            .map(|filter| BoundFilter {
                expression: filter.clone(),
                datasets: vec![NormalizedName::new(ORDERS)],
                origin: at(),
            })
            .collect(),
        date_range: date_range.map(|(field, from, to)| BoundDateRange {
            field: FieldPath {
                dataset: NormalizedName::new(ORDERS),
                field: NormalizedName::new(field),
            },
            from,
            to,
            origin: at(),
        }),
        subgraph: AuthorizedSubgraph::new(datasets, Vec::new()),
        ..BoundQueryParts::default()
    };
    let time = QueryTime {
        time_zone: TimeZoneName::new("UTC"),
        reference_time: None,
        reference_date: None,
    };
    BoundQuery::new(
        Columns::dimensions_then_metrics(Vec::new(), Vec::new()),
        time,
        parts,
    )
}

pub(super) fn query() -> BoundQuery {
    query_of(&[], None, true)
}

pub(super) fn date(year: u16, month: u8, day: u8) -> DateValue {
    DateValue::new(year, month, day).expect("a date")
}

/// `column >= from AND column <= to`, the `date_range` predicate of a date field (QRY-4.2).
pub(super) fn date_range_term(name: &str, from: DateValue, to: DateValue) -> Expression {
    let comparison = |operator, date| Expression::Binary {
        operator,
        left: Box::new(column(name)),
        right: Box::new(Expression::Literal(Literal::Date(date))),
    };
    and(
        comparison(BinaryOperator::GreaterOrEqual, from),
        comparison(BinaryOperator::LessOrEqual, to),
    )
}

pub(super) fn and(left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator: BinaryOperator::And,
        left: Box::new(left),
        right: Box::new(right),
    }
}

/// Calls `test` with the rewrite context of `plan_query` over `plan_model`.
pub(super) fn with_context<Output>(
    plan_model: &InternalModel,
    plan_query: &BoundQuery,
    test: impl FnOnce(&RewriteContext<'_>) -> Output,
) -> Output {
    let catalog = schema();
    let security = anyone();
    let context = RewriteContext {
        query: plan_query,
        model: plan_model,
        schema: &catalog,
        evidence: Evidence::new(plan_model, None),
        security: &security,
    };
    test(&context)
}
