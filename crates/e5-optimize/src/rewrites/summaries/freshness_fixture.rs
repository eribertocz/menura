//! The fixture of the summary freshness tests: a month summary of `ORDERS` and a catalog
//! state whose base partitions, build and check agree.

use menura_contracts::Sha256Hash;
use menura_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::{DateValue, Expression, Identifier, Reference};
use menura_model::internal_model::Field;
use menura_model::internal_model::{
    Dataset, Descriptions, FieldPath, InternalModel, ModelParts, NormalizedName, RelationshipGraph,
    SourceLocation, Summary, SummaryDimension, TypedValue, ValueType,
};
use menura_representations::bound_query::BoundDateRange;
use menura_representations::origin::QueryLocation;
use menura_representations::time::TimeZoneName;
use serde_json::{Value, json};

use super::freshness::{FailedCondition, check_freshness};
use crate::test_support::CURRENT_HASH;

/// A location in the model document.
fn location() -> SourceLocation {
    SourceLocation {
        pointer: "/semantic_model/0/datasets/0".to_owned(),
        line: 1,
        column: 1,
    }
}

/// A hash from its 64 hexadecimal digits.
fn hash(text: &str) -> Sha256Hash {
    Sha256Hash::try_from(text.to_owned()).expect("a valid hash")
}

/// A logical type from its JSON form.
fn logical(text: Value) -> LogicalType {
    serde_json::from_value(text).expect("a logical type")
}

/// The `date` logical type.
pub(super) fn date_type() -> Value {
    json!({"kind": "date"})
}

/// The `timestamp` with time zone logical type.
pub(super) fn instant_type() -> Value {
    json!({"kind": "timestamp", "precision": "us", "with_time_zone": true})
}

/// The field `name` that reads `expression`, of type `logical_type`.
pub(super) fn field(name: &str, expression: Expression, logical_type: Value) -> Field {
    Field {
        name: NormalizedName::new(name),
        expression: Some(expression),
        typed: Some(TypedValue {
            value_type: ValueType::Logical(logical(logical_type)),
            nullable: false,
        }),
        is_time: Some(true),
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

/// The field `name` that reads the column of the same name, of type `logical_type`.
pub(super) fn column_field(name: &str, logical_type: Value) -> Field {
    let reference = Reference {
        qualifier: None,
        name: Identifier::regular(&name.to_lowercase()).expect("a regular identifier"),
    };
    field(name, Expression::Reference(reference), logical_type)
}

/// The dataset `name` of relation `source` with `fields`.
fn dataset(name: &str, source: &str, mut fields: Vec<Field>, summary: bool) -> Dataset {
    fields.sort_by(|left, right| left.name.cmp(&right.name));
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
        summary,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location(),
    }
}

/// A summary dimension.
pub(super) fn dimension(field: &str, base: &str, grain: Option<Grain>) -> SummaryDimension {
    SummaryDimension {
        field: NormalizedName::new(field),
        base: NormalizedName::new(base),
        grain,
    }
}

/// The test fixture: the base `ORDERS` and its summary `ORDERS_BY_MONTH`.
pub(super) struct Fixture {
    pub(super) order_date: Field,
    pub(super) extra_dimensions: Vec<SummaryDimension>,
    pub(super) extra_fields: Vec<Field>,
    pub(super) coverage_grain: Grain,
    pub(super) nullable: bool,
    pub(super) column_type: Value,
    /// The catalog name of the column `ORDER_DATE` reads.
    pub(super) column_name: &'static str,
}

impl Fixture {
    /// A month summary over `ORDERS.ORDER_DATE`, a `date` column that is not nullable.
    pub(super) fn new() -> Self {
        Self {
            order_date: column_field("ORDER_DATE", date_type()),
            extra_dimensions: Vec::new(),
            extra_fields: vec![column_field("SHIPPED_DATE", date_type())],
            coverage_grain: Grain::Month,
            nullable: false,
            column_type: date_type(),
            column_name: "ORDER_DATE",
        }
    }

    /// The summary `ORDERS_BY_MONTH` of `ORDERS`.
    fn summary(&self) -> Summary {
        let mut dimensions = vec![dimension("MONTH", "ORDER_DATE", Some(self.coverage_grain))];
        dimensions.extend(self.extra_dimensions.iter().cloned());
        Summary {
            dataset: NormalizedName::new("ORDERS_BY_MONTH"),
            of: NormalizedName::new("ORDERS"),
            dimensions,
            measures: Vec::new(),
            coverage_field: NormalizedName::new("MONTH"),
            definition_hash: hash(CURRENT_HASH),
            location: location(),
        }
    }

    /// The model with `ORDERS` and `ORDERS_BY_MONTH`.
    fn model(&self) -> InternalModel {
        let mut base_fields = vec![self.order_date.clone()];
        base_fields.extend(self.extra_fields.iter().cloned());
        let parts = ModelParts {
            name: NormalizedName::new("SHOP"),
            descriptions: Descriptions::default(),
            datasets: vec![
                dataset("ORDERS", "shop.main.orders", base_fields, false),
                dataset(
                    "ORDERS_BY_MONTH",
                    "shop.main.orders_by_month",
                    vec![column_field("MONTH", date_type())],
                    true,
                ),
            ],
            relationships: Vec::new(),
            graph: RelationshipGraph::default(),
            metrics: Vec::new(),
            security_context: None,
            summaries: vec![self.summary()],
            checks: Vec::new(),
            unusable: Vec::new(),
            warnings: Vec::new(),
        };
        InternalModel::new(parts, hash(&"0".repeat(64)))
    }

    /// The catalog schema of `ORDERS`.
    fn schema(&self) -> CatalogSchemaDocument {
        let column = |name: &str, logical_type: &Value, nullable: bool| {
            json!({"name": name, "logical_type": logical_type, "physical_type": "DATE",
                   "nullable": nullable})
        };
        let schema = json!({
            "contract": "menura.catalog_schema", "contract_version": "1.1.0",
            "document_id": {"origin": "warehouse:shop", "snapshot": "schema-1"},
            "engine": {"name": "duckdb", "version": "2.0.0"},
            "relations": [
                {"source": "shop.main.orders", "columns": [
                    column(self.column_name, &self.column_type, self.nullable),
                    column("SHIPPED_DATE", &date_type(), false)
                ]},
                {"source": "shop.main.orders_by_month", "columns": [
                    column("MONTH", &date_type(), false)
                ]}
            ]
        });
        serde_json::from_value(schema).expect("a well-formed catalog schema")
    }
}

/// The version token `snapshot`.
pub(super) fn token(snapshot: &str) -> Value {
    json!({"origin": "warehouse:shop", "snapshot": snapshot})
}

/// Period versions: each period with the version named after it.
pub(super) fn versions(periods: &[&str]) -> Value {
    periods
        .iter()
        .map(|period| json!({"period": period, "version": token(period)}))
        .collect()
}

/// The days the tests partition `ORDERS` into.
const DAYS: [&str; 3] = ["2026-09-15", "2026-10-02", "2026-10-31"];

/// A state whose base partitions, build and check all agree on `DAYS`, with coverage
/// `2026-09` to `2026-10`, built in UTC.
pub(super) fn fresh_state() -> Value {
    json!({
        "contract": "menura.catalog_state", "contract_version": "1.1.0",
        "document_id": {"origin": "warehouse:shop", "snapshot": "state-1"},
        "schema_document_id": {"origin": "warehouse:shop", "snapshot": "schema-1"},
        "relations": [
            {"source": "shop.main.orders", "row_count_estimate": 10, "partitions": {
                "column": "ORDER_DATE", "grain": "day", "versions": versions(&DAYS)}},
            {"source": "shop.main.orders_by_month", "row_count_estimate": 2, "aggregate_build": {
                "definition_hash": CURRENT_HASH,
                "coverage": {"from": "2026-09", "to": "2026-10"},
                "built_from": versions(&DAYS),
                "integrity_check": {"status": "passed", "definition_hash": CURRENT_HASH,
                                    "verified": versions(&DAYS)},
                "time_zone": "UTC"}}
        ]
    })
}

/// The base relation in `state`.
pub(super) fn base(state: &mut Value) -> &mut Value {
    &mut state["relations"][0]
}

/// The summary build in `state`.
pub(super) fn build(state: &mut Value) -> &mut Value {
    &mut state["relations"][1]["aggregate_build"]
}

/// A date range on `ORDERS.field`.
pub(super) fn range(field: &str, from: (u16, u8, u8), to: (u16, u8, u8)) -> BoundDateRange {
    BoundDateRange {
        field: FieldPath {
            dataset: NormalizedName::new("ORDERS"),
            field: NormalizedName::new(field),
        },
        from: DateValue::new(from.0, from.1, from.2).expect("a date"),
        to: DateValue::new(to.0, to.1, to.2).expect("a date"),
        origin: QueryLocation::new("/query/nonportable_extensions/0/data/date_range", None),
    }
}

/// The freshness of the fixture's summary in `state`, for a query in `time_zone`.
pub(super) fn freshness(
    fixture: &Fixture,
    state: Value,
    time_zone: &str,
    date_range: Option<&BoundDateRange>,
) -> Result<(), FailedCondition> {
    let state: CatalogStateDocument = serde_json::from_value(state).expect("a catalog state");
    check_freshness(
        &fixture.summary(),
        &fixture.model(),
        &fixture.schema(),
        &state,
        &TimeZoneName::new(time_zone),
        date_range,
    )
}
