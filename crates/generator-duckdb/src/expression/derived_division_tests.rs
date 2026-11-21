//! Golden SQL of `/` and `%` inside a derived metric (E6-4.6, E6-4.6a, DER-3.1).

use menura_contracts::types::catalog::{CatalogSchemaContract, CatalogSchemaDocument, Engine};
use menura_contracts::types::common::{IntegerBits, LogicalType, Token};
use menura_contracts::{ContractKind, Sha256Hash};
use menura_model::internal_model::{
    Descriptions, DivisionByZero, InternalModel, ModelParts, NormalizedName, RelationshipGraph,
    SourceLocation,
};
use menura_representations::logical_plan::{OutputSchema, SchemaColumn};
use menura_representations::names::ColumnName;
use menura_representations::origin::{Origin, QueryLocation};
use menura_representations::time::{QueryTime, TimeZoneName};
use menura_security::admit_security_context;

use super::{Columns, Environment, ExpressionWriter};
use crate::parameters::Parameters;

/// The input columns: `REVENUE` and `ORDERS`, 64-bit integers, and `UNITS`, a 128-bit integer.
fn input_schema() -> OutputSchema {
    let column = |name: &str, bits| SchemaColumn {
        name: ColumnName::new(name),
        logical_type: LogicalType::Integer { bits, signed: true },
        nullable: true,
    };
    OutputSchema::new(vec![
        column("REVENUE", IntegerBits::SixtyFour),
        column("ORDERS", IntegerBits::SixtyFour),
        column("UNITS", IntegerBits::OneHundredTwentyEight),
    ])
}

fn empty_model() -> InternalModel {
    let parts = ModelParts {
        name: NormalizedName::new("SALES"),
        descriptions: Descriptions::default(),
        datasets: Vec::new(),
        relationships: Vec::new(),
        graph: RelationshipGraph::default(),
        metrics: Vec::new(),
        security_context: None,
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    InternalModel::new(parts, Sha256Hash::try_from("0".repeat(64)).expect("a hash"))
}

fn empty_catalog() -> CatalogSchemaDocument {
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
        relations: Vec::new(),
    }
}

/// The SQL of `formula` over `input_schema`, from `origin`, inside the derived metric `metric`
/// with `rule`, or outside any derived metric when `metric` is `None`.
fn written(
    formula: &str,
    origin: &Origin,
    metric: Option<(&NormalizedName, DivisionByZero)>,
) -> (String, usize) {
    let catalog = empty_catalog();
    let model = empty_model();
    let security = admit_security_context(
        r#"{"contract": "menura.security_context", "contract_version": "1.1.0",
            "subject": "user-7f3a", "roles": [], "attributes": {}}"#,
        None,
    )
    .expect("an admitted context");
    let time = QueryTime {
        time_zone: TimeZoneName::new("Europe/Madrid"),
        reference_date: None,
        reference_time: None,
    };
    let environment = Environment {
        catalog: &catalog,
        model: &model,
        security: &security,
        time: &time,
    };
    let schema = input_schema();
    let mut parameters = Parameters::default();
    let mut writer = ExpressionWriter::new(
        &environment,
        Columns::Input(&schema),
        origin,
        &mut parameters,
    );
    if let Some((name, rule)) = metric {
        writer = writer.in_derived_metric(name, rule);
    }
    let expression = menura_expressions::parse(formula).expect("an expression");
    let sql = writer.write(&expression).expect("SQL");
    (sql, parameters.into_list().len())
}

fn from_model() -> Origin {
    Origin::Model(SourceLocation {
        pointer: "/semantic_model/0/metrics/0".to_owned(),
        line: 1,
        column: 1,
    })
}

fn from_query() -> Origin {
    Origin::Query(QueryLocation::new("/query/metrics/0", None))
}

fn golden(sql: &str, expected: &str) {
    assert!(
        sql == expected.trim_end(),
        "the SQL differs from its golden file; actual SQL:\n{sql}"
    );
}

#[test]
fn e6_4_6_derived_division_with_null_is_the_division_outside_derived_metrics() {
    let metric = NormalizedName::new("REVENUE_PER_ORDER");
    let outside = written("REVENUE / ORDERS", &from_model(), None);
    let inside = written(
        "REVENUE / ORDERS",
        &from_model(),
        Some((&metric, DivisionByZero::Null)),
    );
    assert_eq!(inside, outside);
    golden(
        &inside.0,
        include_str!("../../tests/golden/e6_4_6_derived_division_null.sql"),
    );
}

#[test]
fn e6_4_6_derived_modulo_with_null_is_the_modulo_outside_derived_metrics() {
    let metric = NormalizedName::new("UNITS_LEFT");
    let outside = written("UNITS % ORDERS", &from_model(), None);
    let inside = written(
        "UNITS % ORDERS",
        &from_model(),
        Some((&metric, DivisionByZero::Null)),
    );
    assert_eq!(inside, outside);
    golden(
        &inside.0,
        include_str!("../../tests/golden/e6_4_6_derived_modulo_null.sql"),
    );
}

#[test]
fn e6_4_6a_derived_division_with_error_raises_rder_01() {
    let metric = NormalizedName::new("REVENUE_PER_ORDER");
    let (sql, _) = written(
        "REVENUE / ORDERS",
        &from_model(),
        Some((&metric, DivisionByZero::Error)),
    );
    golden(
        &sql,
        include_str!("../../tests/golden/e6_4_6a_derived_division_error.sql"),
    );
}

#[test]
fn e6_4_6a_derived_modulo_with_error_raises_rder_01() {
    let metric = NormalizedName::new("UNITS_LEFT");
    let (sql, _) = written(
        "UNITS % ORDERS",
        &from_model(),
        Some((&metric, DivisionByZero::Error)),
    );
    golden(
        &sql,
        include_str!("../../tests/golden/e6_4_6a_derived_modulo_error.sql"),
    );
}

#[test]
fn e6_4_6a_every_division_of_the_formula_raises_and_names_the_metric() {
    let metric = NormalizedName::new("MIX");
    let (sql, _) = written(
        "(REVENUE / ORDERS) % ORDERS",
        &from_model(),
        Some((&metric, DivisionByZero::Error)),
    );
    let raised = "error('RDER-01: division by zero in metric MIX')";
    assert_eq!(sql.matches(raised).count(), 2, "{sql}");
    assert!(!sql.contains("THEN NULL"), "{sql}");
}

#[test]
fn der_3_1_on_division_by_zero_chooses_the_result_of_a_zero_divisor() {
    let metric = NormalizedName::new("REVENUE_PER_ORDER");
    let rule = |rule| written("REVENUE / ORDERS", &from_model(), Some((&metric, rule))).0;
    assert!(rule(DivisionByZero::Null).contains("THEN NULL ELSE"));
    assert!(rule(DivisionByZero::Error).contains("THEN error('RDER-01: "));
}

#[test]
fn ext_1_the_metric_name_in_the_error_text_is_a_safe_string_literal() {
    let metric = NormalizedName::new("O'BRIEN RATIO");
    let (sql, _) = written(
        "REVENUE / ORDERS",
        &from_model(),
        Some((&metric, DivisionByZero::Error)),
    );
    assert!(
        sql.contains("error('RDER-01: division by zero in metric O''BRIEN RATIO')"),
        "{sql}"
    );
}

#[test]
fn e6_4_11_the_error_text_is_a_literal_even_in_a_query_node() {
    let metric = NormalizedName::new("REVENUE_PER_ORDER");
    let (sql, parameters) = written(
        "REVENUE / ORDERS",
        &from_query(),
        Some((&metric, DivisionByZero::Error)),
    );
    assert_eq!(parameters, 0);
    assert!(
        sql.contains("error('RDER-01: division by zero in metric REVENUE_PER_ORDER')"),
        "{sql}"
    );
}
