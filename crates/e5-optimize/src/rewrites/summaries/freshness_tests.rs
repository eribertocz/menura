//! Unit tests of summary freshness (CAT-7.2, CAT-7.2.1, SUM-3.7).

use menura_contracts::Sha256Hash;
use menura_contracts::types::catalog::{CatalogSchemaDocument, CatalogStateDocument};
use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::{DateValue, Expression, Identifier, Literal, Reference};
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
use crate::test_support::{CURRENT_HASH, OTHER_HASH};

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
fn date_type() -> Value {
    json!({"kind": "date"})
}

/// The `timestamp` with time zone logical type.
fn instant_type() -> Value {
    json!({"kind": "timestamp", "precision": "us", "with_time_zone": true})
}

/// The field `name` that reads `expression`, of type `logical_type`.
fn field(name: &str, expression: Expression, logical_type: Value) -> Field {
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
fn column_field(name: &str, logical_type: Value) -> Field {
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
fn dimension(field: &str, base: &str, grain: Option<Grain>) -> SummaryDimension {
    SummaryDimension {
        field: NormalizedName::new(field),
        base: NormalizedName::new(base),
        grain,
    }
}

/// The test fixture: the base `ORDERS` and its summary `ORDERS_BY_MONTH`.
struct Fixture {
    order_date: Field,
    extra_dimensions: Vec<SummaryDimension>,
    extra_fields: Vec<Field>,
    coverage_grain: Grain,
    nullable: bool,
    column_type: Value,
}

impl Fixture {
    /// A month summary over `ORDERS.ORDER_DATE`, a `date` column that is not nullable.
    fn new() -> Self {
        Self {
            order_date: column_field("ORDER_DATE", date_type()),
            extra_dimensions: Vec::new(),
            extra_fields: vec![column_field("SHIPPED_DATE", date_type())],
            coverage_grain: Grain::Month,
            nullable: false,
            column_type: date_type(),
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
                    column("ORDER_DATE", &self.column_type, self.nullable),
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
fn token(snapshot: &str) -> Value {
    json!({"origin": "warehouse:shop", "snapshot": snapshot})
}

/// Period versions: each period with the version named after it.
fn versions(periods: &[&str]) -> Value {
    periods
        .iter()
        .map(|period| json!({"period": period, "version": token(period)}))
        .collect()
}

/// The days the tests partition `ORDERS` into.
const DAYS: [&str; 3] = ["2026-09-15", "2026-10-02", "2026-10-31"];

/// A state whose base partitions, build and check all agree on `DAYS`, with coverage
/// `2026-09` to `2026-10`, built in UTC.
fn fresh_state() -> Value {
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
fn base(state: &mut Value) -> &mut Value {
    &mut state["relations"][0]
}

/// The summary build in `state`.
fn build(state: &mut Value) -> &mut Value {
    &mut state["relations"][1]["aggregate_build"]
}

/// A date range on `ORDERS.field`.
fn range(field: &str, from: (u16, u8, u8), to: (u16, u8, u8)) -> BoundDateRange {
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
fn freshness(
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

#[test]
fn cat_7_2_a_month_summary_over_day_partitions_is_valid() {
    assert_eq!(
        freshness(&Fixture::new(), fresh_state(), "UTC", None),
        Ok(())
    );
}

#[test]
fn cat_7_2_condition_1_fails_when_the_base_is_not_partitioned() {
    let mut state = fresh_state();
    *base(&mut state) =
        json!({"source": "shop.main.orders", "row_count_estimate": 10, "version": token("v1")});
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_fails_when_the_summary_grain_does_not_derive_from_the_partitions() {
    let mut state = fresh_state();
    base(&mut state)["partitions"]["grain"] = json!("week");
    base(&mut state)["partitions"]["versions"] = versions(&["2026-W38"]);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_fails_when_the_partitions_are_on_another_column() {
    let mut state = fresh_state();
    base(&mut state)["partitions"]["column"] = json!("SHIPPED_DATE");
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_fails_when_the_coverage_field_is_an_expression() {
    let mut fixture = Fixture::new();
    fixture.order_date = field(
        "ORDER_DATE",
        Expression::Literal(Literal::Boolean(true)),
        date_type(),
    );
    let result = freshness(&fixture, fresh_state(), "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_fails_when_the_partition_column_has_a_time_zone() {
    let mut fixture = Fixture::new();
    fixture.column_type = instant_type();
    let result = freshness(&fixture, fresh_state(), "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));
}

#[test]
fn cat_7_2_condition_1_a_nullable_partition_column_needs_a_date_range_on_coverage() {
    let mut fixture = Fixture::new();
    fixture.nullable = true;
    let result = freshness(&fixture, fresh_state(), "UTC", None);
    assert_eq!(result, Err(FailedCondition::Partitions));

    let on_other = range("SHIPPED_DATE", (2026, 9, 1), (2026, 10, 31));
    let result = freshness(&fixture, fresh_state(), "UTC", Some(&on_other));
    assert_eq!(result, Err(FailedCondition::Partitions));

    let on_coverage = range("ORDER_DATE", (2026, 9, 1), (2026, 10, 31));
    let result = freshness(&fixture, fresh_state(), "UTC", Some(&on_coverage));
    assert_eq!(result, Ok(()));
}

#[test]
fn cat_7_2_condition_2_fails_when_a_period_is_outside_coverage() {
    let mut state = fresh_state();
    build(&mut state)["coverage"] = json!({"from": "2026-10", "to": "2026-10"});
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Coverage));
}

#[test]
fn cat_7_2_condition_3_fails_when_a_base_period_is_added_after_the_build() {
    let mut state = fresh_state();
    base(&mut state)["partitions"]["versions"] =
        versions(&["2026-09-15", "2026-09-20", "2026-10-02", "2026-10-31"]);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::BuiltFrom));
}

#[test]
fn cat_7_2_condition_3_fails_when_a_base_version_changed() {
    let mut state = fresh_state();
    base(&mut state)["partitions"]["versions"][1]["version"] = token("rewritten");
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::BuiltFrom));
}

#[test]
fn cat_7_2_condition_3_fails_when_built_from_has_another_grain() {
    let mut state = fresh_state();
    build(&mut state)["built_from"] = versions(&["2026-09", "2026-10"]);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::BuiltFrom));
}

#[test]
fn cat_7_2_condition_4_fails_when_the_definition_changed() {
    let mut state = fresh_state();
    build(&mut state)["definition_hash"] = json!(OTHER_HASH);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::DefinitionHash));
}

#[test]
fn cat_7_2_condition_5_fails_when_an_instant_dimension_was_built_in_another_zone() {
    let mut fixture = Fixture::new();
    fixture
        .extra_fields
        .push(column_field("ORDERED_AT", instant_type()));
    fixture
        .extra_dimensions
        .push(dimension("ORDERED_DAY", "ORDERED_AT", Some(Grain::Day)));
    let mut state = fresh_state();
    build(&mut state)["time_zone"] = json!("Europe/Madrid");
    let result = freshness(&fixture, state.clone(), "UTC", None);
    assert_eq!(result, Err(FailedCondition::TimeZone));
    assert_eq!(freshness(&fixture, state, "Europe/Madrid", None), Ok(()));
}

#[test]
fn cat_7_2_condition_5_compares_time_zones_by_code_points() {
    let mut fixture = Fixture::new();
    fixture
        .extra_fields
        .push(column_field("ORDERED_AT", instant_type()));
    fixture
        .extra_dimensions
        .push(dimension("ORDERED_DAY", "ORDERED_AT", Some(Grain::Day)));
    let result = freshness(&fixture, fresh_state(), "Etc/UTC", None);
    assert_eq!(result, Err(FailedCondition::TimeZone));
}

#[test]
fn cat_7_2_condition_5_holds_without_an_instant_dimension() {
    let mut state = fresh_state();
    build(&mut state)["time_zone"] = json!("Europe/Madrid");
    assert_eq!(freshness(&Fixture::new(), state, "UTC", None), Ok(()));
}

#[test]
fn cat_7_2_condition_6_fails_when_the_check_did_not_pass() {
    let mut state = fresh_state();
    build(&mut state)["integrity_check"]["status"] = json!("failed");
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::IntegrityCheck));
}

#[test]
fn cat_7_2_condition_6_fails_when_the_check_ran_on_another_definition() {
    let mut state = fresh_state();
    build(&mut state)["integrity_check"]["definition_hash"] = json!(OTHER_HASH);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::IntegrityCheck));
}

#[test]
fn cat_7_2_condition_6_fails_when_the_check_missed_a_base_period() {
    let mut state = fresh_state();
    build(&mut state)["integrity_check"]["verified"] = versions(&["2026-09-15", "2026-10-02"]);
    let result = freshness(&Fixture::new(), state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::IntegrityCheck));
}

#[test]
fn cat_7_2_1_a_period_listed_by_neither_side_needs_no_evidence() {
    let mut state = fresh_state();
    // 2026-09-16 is in Q (inside `2026-09`) and listed by no side; so is all of a later month
    // that no side lists.
    build(&mut state)["coverage"] = json!({"from": "2026-09", "to": "2026-10"});
    let query = range("ORDER_DATE", (2026, 9, 1), (2026, 12, 31));
    let result = freshness(&Fixture::new(), state, "UTC", Some(&query));
    assert_eq!(result, Ok(()));
}

#[test]
fn cat_7_2_a_date_range_on_coverage_restricts_p() {
    let mut state = fresh_state();
    let later = ["2026-09-15", "2026-10-02", "2026-10-31", "2026-11-03"];
    base(&mut state)["partitions"]["versions"] = versions(&later);
    assert_eq!(
        freshness(&Fixture::new(), state.clone(), "UTC", None),
        Err(FailedCondition::Coverage)
    );

    let on_other = range("SHIPPED_DATE", (2026, 9, 1), (2026, 10, 31));
    assert_eq!(
        freshness(&Fixture::new(), state.clone(), "UTC", Some(&on_other)),
        Err(FailedCondition::Coverage)
    );

    let on_coverage = range("ORDER_DATE", (2026, 9, 1), (2026, 10, 31));
    let result = freshness(&Fixture::new(), state, "UTC", Some(&on_coverage));
    assert_eq!(result, Ok(()));
}

#[test]
fn cat_7_2_a_week_summary_groups_day_partitions_into_iso_weeks() {
    let mut fixture = Fixture::new();
    fixture.coverage_grain = Grain::Week;
    let mut state = fresh_state();
    let days = ["2021-01-03", "2021-01-04"];
    base(&mut state)["partitions"]["versions"] = versions(&days);
    build(&mut state)["built_from"] = versions(&days);
    build(&mut state)["integrity_check"]["verified"] = versions(&days);
    build(&mut state)["coverage"] = json!({"from": "2020-W53", "to": "2021-W01"});
    assert_eq!(freshness(&fixture, state.clone(), "UTC", None), Ok(()));

    build(&mut state)["coverage"] = json!({"from": "2021-W01", "to": "2021-W01"});
    let result = freshness(&fixture, state, "UTC", None);
    assert_eq!(result, Err(FailedCondition::Coverage));
}
