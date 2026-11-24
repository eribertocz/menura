//! Values shared by the unit tests of this crate.

use menura_contracts::Sha256Hash;
use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, Literal};
use menura_model::internal_model::{
    CheckKind, Dataset, Descriptions, InternalModel, ModelParts, NormalizedName, RecordedCheck,
    Relationship, RelationshipGraph, SourceLocation,
};
use menura_representations::bound_query::{BoundQuery, BoundQueryParts};
use menura_representations::columns::Columns;
use menura_representations::logical_plan::{Operation, OutputSchema, PlanNode, SchemaColumn};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use menura_representations::time::{QueryTime, TimeZoneName};
use menura_security::{AdmittedSecurityContext, admit_security_context};

/// The relationship from `ORDERS` to `CUSTOMERS`.
pub(crate) const RELATIONSHIP: &str = "ORDERS_TO_CUSTOMERS";
/// The definition hash of both recorded checks of `RELATIONSHIP`.
pub(crate) const CURRENT_HASH: &str =
    "1111111111111111111111111111111111111111111111111111111111111111";
/// A definition hash of no current check.
pub(crate) const OTHER_HASH: &str =
    "2222222222222222222222222222222222222222222222222222222222222222";

/// A location in the model document.
pub(crate) fn location(pointer: &str) -> SourceLocation {
    SourceLocation {
        pointer: pointer.to_owned(),
        line: 1,
        column: 1,
    }
}

/// A hash from its 64 hexadecimal digits.
pub(crate) fn hash(text: &str) -> Sha256Hash {
    Sha256Hash::try_from(text.to_owned()).expect("a valid hash")
}

/// The dataset `name` of relation `source`, without fields.
pub(crate) fn dataset(name: &str, source: &str) -> Dataset {
    Dataset {
        name: NormalizedName::new(name),
        source: source.to_owned(),
        fields: Vec::new(),
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

/// The recorded check of `kind` for `RELATIONSHIP`, with `CURRENT_HASH`.
fn check(kind: CheckKind) -> RecordedCheck {
    RecordedCheck {
        check_id: format!("{}:{RELATIONSHIP}", kind.as_str()),
        kind,
        target: RELATIONSHIP.to_owned(),
        definition_hash: hash(CURRENT_HASH),
    }
}

/// The model: `ORDERS` (`shop.orders`) related to `CUSTOMERS` (`shop.customers`)
/// by `RELATIONSHIP`, whose `key_uniqueness` and `orphan_keys` checks are recorded.
pub(crate) fn shop_model() -> InternalModel {
    let parts = ModelParts {
        name: NormalizedName::new("SHOP"),
        descriptions: Descriptions::default(),
        datasets: vec![
            dataset("CUSTOMERS", "shop.customers"),
            dataset("ORDERS", "shop.orders"),
        ],
        relationships: vec![Relationship {
            name: NormalizedName::new(RELATIONSHIP),
            from: NormalizedName::new("ORDERS"),
            to: NormalizedName::new("CUSTOMERS"),
            from_columns: vec![NormalizedName::new("CUSTOMER_ID")],
            to_columns: vec![NormalizedName::new("CUSTOMER_ID")],
            descriptions: Descriptions::default(),
            query_errors: Vec::new(),
            location: location("/semantic_model/0/relationships/0"),
        }],
        graph: RelationshipGraph::default(),
        metrics: Vec::new(),
        security_context: None,
        summaries: Vec::new(),
        checks: vec![
            check(CheckKind::KeyUniqueness),
            check(CheckKind::OrphanKeys),
        ],
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    InternalModel::new(parts, hash(&"0".repeat(64)))
}

/// The catalog schema of `shop_model`.
pub(crate) fn shop_schema() -> CatalogSchemaDocument {
    let integer = r#"{"kind": "integer", "bits": 32, "signed": true}"#;
    let text = format!(
        r#"{{"contract": "menura.catalog_schema", "contract_version": "1.1.0",
            "document_id": {{"origin": "warehouse:shop", "snapshot": "schema-1"}},
            "engine": {{"name": "duckdb", "version": "2.0.0"}},
            "relations": [
              {{"source": "shop.customers", "columns": [{{"name": "CUSTOMER_ID",
                "logical_type": {integer}, "physical_type": "INTEGER", "nullable": false}}]}},
              {{"source": "shop.orders", "columns": [{{"name": "CUSTOMER_ID",
                "logical_type": {integer}, "physical_type": "INTEGER", "nullable": false}}]}}
            ]}}"#
    );
    serde_json::from_str(&text).expect("a well-formed catalog schema")
}

/// A query with no output column, in UTC.
pub(crate) fn empty_query() -> BoundQuery {
    let time = QueryTime {
        time_zone: TimeZoneName::new("UTC"),
        reference_time: None,
        reference_date: None,
    };
    BoundQuery::new(
        Columns::dimensions_then_metrics(Vec::new(), Vec::new()),
        time,
        BoundQueryParts::default(),
    )
}

/// The admitted security context of a user with no role.
pub(crate) fn anyone() -> AdmittedSecurityContext {
    let text = r#"{"contract": "menura.security_context", "contract_version": "1.1.0",
        "subject": "user-1", "roles": [], "attributes": {}}"#;
    admit_security_context(text, None).expect("an admissible context")
}

/// The plan `Filter(Scan(ORDERS), TRUE)`: a home `Scan` with a row-policy filter.
pub(crate) fn filtered_scan() -> PlanNode {
    let schema = OutputSchema::new(vec![SchemaColumn {
        name: ColumnName::new("CUSTOMER_ID"),
        logical_type: serde_json::from_str::<LogicalType>(
            r#"{"kind": "integer", "bits": 32, "signed": true}"#,
        )
        .expect("a logical type"),
        nullable: false,
    }]);
    let origin = Origin::Model(location("/semantic_model/0/datasets/1"));
    let scan = PlanNode::new(
        Operation::Scan {
            dataset: NormalizedName::new("ORDERS"),
            row_identity: None,
        },
        schema.clone(),
        origin.clone(),
    );
    PlanNode::new(
        Operation::Filter {
            input: Box::new(scan),
            predicate: Expression::Literal(Literal::Boolean(true)),
        },
        schema,
        origin,
    )
}
