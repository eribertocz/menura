//! Unit tests of the deduplication rewrite (E5-3.2, E5-3.2a, E5-3.8, E5-5).

use menura_contracts::types::catalog::CatalogStateDocument;
use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, Literal};
use menura_model::internal_model::{
    CheckKind, Descriptions, InternalModel, NormalizedName, RecordedCheck, Relationship,
};
use menura_representations::logical_plan::{
    JoinCondition, JoinKind, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;
use serde_json::{Value, json};

use super::deduplication::rewrite;
use crate::context::RewriteContext;
use crate::evidence::Evidence;
use crate::record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};
use crate::test_support::{
    CURRENT_HASH, OTHER_HASH, RELATIONSHIP, anyone, dataset, empty_query, hash, location,
    shop_model, shop_schema,
};

/// The relationship from `CUSTOMERS` to `REGIONS`, added by `two_join_model`.
const SECOND_RELATIONSHIP: &str = "CUSTOMERS_TO_REGIONS";
/// The synthesized row identity of the home `Scan` (E4-4.0f).
const IDENTITY: &str = "menura.row_identity";

/// A 32-bit signed integer column `name`.
fn column(name: &str, nullable: bool) -> SchemaColumn {
    SchemaColumn {
        name: ColumnName::new(name),
        logical_type: serde_json::from_str::<LogicalType>(
            r#"{"kind": "integer", "bits": 32, "signed": true}"#,
        )
        .expect("a logical type"),
        nullable,
    }
}

/// A plan node with `columns`, from the `ORDERS` dataset of the model.
fn node(operation: Operation, columns: Vec<SchemaColumn>) -> PlanNode {
    let origin = Origin::Model(location("/semantic_model/0/datasets/1"));
    PlanNode::new(operation, OutputSchema::new(columns), origin)
}

/// The visible rows of the home dataset `ORDERS`: `Filter(Scan(ORDERS), TRUE)`, with the
/// synthesized row identity when `identity`.
fn home(identity: bool) -> PlanNode {
    let mut columns = vec![column("AMOUNT", false), column("CUSTOMER_ID", false)];
    if identity {
        columns.push(column(IDENTITY, false));
    }
    let scan = node(
        Operation::Scan {
            dataset: NormalizedName::new("ORDERS"),
            row_identity: identity.then(|| ColumnName::new(IDENTITY)),
        },
        columns.clone(),
    );
    node(
        Operation::Filter {
            input: Box::new(scan),
            predicate: Expression::Literal(Literal::Boolean(true)),
        },
        columns,
    )
}

/// The `Scan` of the joined dataset `dataset`, with the columns `keys` under `prefix`.
fn joined_scan(dataset: &str, prefix: &str, keys: &[&str]) -> PlanNode {
    let columns = keys
        .iter()
        .map(|key| column(&format!("{prefix}.{key}"), false))
        .collect();
    node(
        Operation::Scan {
            dataset: NormalizedName::new(dataset),
            row_identity: None,
        },
        columns,
    )
}

/// The left `Join` of `left` with `right` along `relationship`, on the pair of columns `keys`.
fn join(
    left: PlanNode,
    right: PlanNode,
    relationship: &str,
    keys: (&str, &str),
    prefix: &str,
) -> PlanNode {
    let mut columns = left.schema().columns().to_vec();
    columns.extend(right.schema().columns().iter().cloned().map(|mut column| {
        column.nullable = true;
        column
    }));
    node(
        Operation::Join {
            left: Box::new(left),
            right: Box::new(right),
            relationship: NormalizedName::new(relationship),
            kind: JoinKind::Left,
            on: vec![JoinCondition {
                left: ColumnName::new(keys.0),
                right: ColumnName::new(keys.1),
            }],
            right_prefix: ColumnPrefix::new(&format!("{prefix}.")),
        },
        columns,
    )
}

/// `home(identity)` joined with `CUSTOMERS` along `RELATIONSHIP`.
fn one_join(identity: bool) -> PlanNode {
    join(
        home(identity),
        joined_scan("CUSTOMERS", RELATIONSHIP, &["CUSTOMER_ID", "REGION_ID"]),
        RELATIONSHIP,
        ("CUSTOMER_ID", "ORDERS_TO_CUSTOMERS.CUSTOMER_ID"),
        RELATIONSHIP,
    )
}

/// `one_join(identity)` joined with `REGIONS` along `SECOND_RELATIONSHIP`.
fn two_joins(identity: bool) -> PlanNode {
    join(
        one_join(identity),
        joined_scan("REGIONS", SECOND_RELATIONSHIP, &["REGION_ID"]),
        SECOND_RELATIONSHIP,
        (
            "ORDERS_TO_CUSTOMERS.REGION_ID",
            "CUSTOMERS_TO_REGIONS.REGION_ID",
        ),
        SECOND_RELATIONSHIP,
    )
}

/// `Limit(Deduplicate(input), 10)`: the `Deduplicate` is at path `[0]`.
fn deduplicated(input: PlanNode) -> PlanNode {
    let columns = input.schema().columns().to_vec();
    let deduplicate = node(
        Operation::Deduplicate {
            input: Box::new(input),
            group: Vec::new(),
            identity: vec![ColumnName::new(IDENTITY)],
        },
        columns.clone(),
    );
    limited(deduplicate)
}

/// `Limit(input, 10)`, whose output has the columns of `input` but the row identity.
fn limited(input: PlanNode) -> PlanNode {
    let columns = input
        .schema()
        .columns()
        .iter()
        .filter(|column| column.name.as_str() != IDENTITY)
        .cloned()
        .collect();
    node(
        Operation::Limit {
            input: Box::new(input),
            rows: 10,
        },
        columns,
    )
}

/// `shop_model` with the dataset `REGIONS` (`shop.main.regions`) and `SECOND_RELATIONSHIP`
/// from `CUSTOMERS` to it, each kept in name order, whose `key_uniqueness` check is recorded with `CURRENT_HASH`.
fn two_join_model() -> InternalModel {
    let mut parts = shop_model().parts().clone();
    parts.datasets.push(dataset("REGIONS", "shop.main.regions"));
    parts.relationships.insert(
        0,
        Relationship {
            name: NormalizedName::new(SECOND_RELATIONSHIP),
            from: NormalizedName::new("CUSTOMERS"),
            to: NormalizedName::new("REGIONS"),
            from_columns: vec![NormalizedName::new("REGION_ID")],
            to_columns: vec![NormalizedName::new("REGION_ID")],
            descriptions: Descriptions::default(),
            query_errors: Vec::new(),
            location: location("/semantic_model/0/relationships/1"),
        },
    );
    parts.checks.push(RecordedCheck {
        check_id: format!("key_uniqueness:{SECOND_RELATIONSHIP}"),
        kind: CheckKind::KeyUniqueness,
        target: SECOND_RELATIONSHIP.to_owned(),
        definition_hash: hash(CURRENT_HASH),
    });
    InternalModel::new(parts, hash(&"0".repeat(64)))
}

/// The version token `snapshot` of the test origin.
fn token(snapshot: &str) -> Value {
    json!({"origin": "warehouse:shop", "snapshot": snapshot})
}

/// A `key_uniqueness` result for `target` of `definition_hash` and `status`, verified on the
/// `to` relation `source` at version `version`.
fn uniqueness(target: &str, definition_hash: &str, status: &str, verified: (&str, &str)) -> Value {
    json!({
        "target": target,
        "check": "key_uniqueness",
        "definition_hash": definition_hash,
        "status": status,
        "verified": [{"source": verified.0, "version": token(verified.1)}]
    })
}

/// The passing, current `key_uniqueness` result of `RELATIONSHIP`.
fn customers_unique() -> Value {
    uniqueness(
        RELATIONSHIP,
        CURRENT_HASH,
        "passed",
        ("shop.main.customers", "customers-1"),
    )
}

/// The passing, current `key_uniqueness` result of `SECOND_RELATIONSHIP`.
fn regions_unique() -> Value {
    uniqueness(
        SECOND_RELATIONSHIP,
        CURRENT_HASH,
        "passed",
        ("shop.main.regions", "regions-1"),
    )
}

/// A state where every relation is at version `<name>-1`, with the results `checks`.
fn state(checks: &[Value]) -> CatalogStateDocument {
    let relation = |name: &str| {
        json!({"source": format!("shop.main.{name}"), "row_count_estimate": 1,
            "version": token(&format!("{name}-1"))})
    };
    serde_json::from_value(json!({
        "contract": "menura.catalog_state",
        "contract_version": "1.1.0",
        "document_id": token("state-1"),
        "schema_document_id": token("schema-1"),
        "relations": [relation("orders"), relation("customers"), relation("regions")],
        "checks": checks
    }))
    .expect("a well-formed catalog state")
}

/// The rewrite of `plan` against `model` and `state`, with its records.
fn run(
    plan: PlanNode,
    model: &InternalModel,
    state: Option<&CatalogStateDocument>,
) -> (PlanNode, Vec<RewriteRecord>) {
    let (query, schema, security) = (empty_query(), shop_schema(), anyone());
    let context = RewriteContext {
        query: &query,
        model,
        schema: &schema,
        evidence: Evidence::new(model, state),
        security: &security,
    };
    let mut records = Vec::new();
    let plan = rewrite(plan, &context, &mut records);
    (plan, records)
}

/// The record of the `Deduplicate` at path `[0]`, with `outcome`.
fn record(outcome: RewriteOutcome) -> RewriteRecord {
    RewriteRecord {
        rule: RewriteRule::SkipDeduplication,
        subject: RewriteSubject::Node(NodePath::new(vec![0])),
        outcome,
    }
}

/// The record of a `Deduplicate` kept for want of evidence for `relationship`.
fn kept_for(relationship: &str) -> RewriteRecord {
    record(RewriteOutcome::Rejected {
        failed_condition: format!(
            "no passing key_uniqueness evidence for relationship {relationship} (E5-3.2a)"
        ),
    })
}

#[test]
fn e5_3_2_evidence_for_the_only_join_removes_the_deduplicate_and_the_row_identity() {
    let state = state(&[customers_unique()]);

    let (plan, records) = run(deduplicated(one_join(true)), &shop_model(), Some(&state));

    assert_eq!(plan, limited(one_join(false)));
    assert_eq!(records, [record(RewriteOutcome::Applied)]);
}

#[test]
fn e5_3_2a_evidence_for_every_join_removes_the_deduplicate() {
    let state = state(&[customers_unique(), regions_unique()]);

    let (plan, records) = run(
        deduplicated(two_joins(true)),
        &two_join_model(),
        Some(&state),
    );

    assert_eq!(plan, limited(two_joins(false)));
    assert_eq!(records, [record(RewriteOutcome::Applied)]);
}

#[test]
fn e5_3_2a_one_join_without_evidence_keeps_the_deduplicate() {
    let state = state(&[customers_unique()]);
    let plan = deduplicated(two_joins(true));

    let (rewritten, records) = run(plan.clone(), &two_join_model(), Some(&state));

    assert_eq!(rewritten, plan);
    assert_eq!(records, [kept_for(SECOND_RELATIONSHIP)]);
}

#[test]
fn e5_3_8_the_first_relationship_without_evidence_is_recorded() {
    let state = state(&[]);
    let plan = deduplicated(two_joins(true));

    let (rewritten, records) = run(plan.clone(), &two_join_model(), Some(&state));

    assert_eq!(rewritten, plan);
    assert_eq!(records, [kept_for(RELATIONSHIP)]);
}

#[test]
fn e5_3_2a_failed_evidence_keeps_the_deduplicate() {
    let failed_result = uniqueness(
        RELATIONSHIP,
        CURRENT_HASH,
        "failed",
        ("shop.main.customers", "customers-1"),
    );
    let state = state(&[failed_result]);
    let plan = deduplicated(one_join(true));

    let (rewritten, records) = run(plan.clone(), &shop_model(), Some(&state));

    assert_eq!(rewritten, plan);
    assert_eq!(records, [kept_for(RELATIONSHIP)]);
}

#[test]
fn e5_3_2a_evidence_on_a_stale_version_keeps_the_deduplicate() {
    let stale_result = uniqueness(
        RELATIONSHIP,
        CURRENT_HASH,
        "passed",
        ("shop.main.customers", "customers-0"),
    );
    let state = state(&[stale_result]);
    let plan = deduplicated(one_join(true));

    let (rewritten, records) = run(plan.clone(), &shop_model(), Some(&state));

    assert_eq!(rewritten, plan);
    assert_eq!(records, [kept_for(RELATIONSHIP)]);
}

#[test]
fn e5_3_2a_evidence_of_another_definition_hash_keeps_the_deduplicate() {
    let other_result = uniqueness(
        RELATIONSHIP,
        OTHER_HASH,
        "passed",
        ("shop.main.customers", "customers-1"),
    );
    let state = state(&[other_result]);
    let plan = deduplicated(one_join(true));

    let (rewritten, records) = run(plan.clone(), &shop_model(), Some(&state));

    assert_eq!(rewritten, plan);
    assert_eq!(records, [kept_for(RELATIONSHIP)]);
}

#[test]
fn e5_5_without_a_state_the_deduplicate_is_kept() {
    let plan = deduplicated(one_join(true));

    let (rewritten, records) = run(plan.clone(), &shop_model(), None);

    assert_eq!(rewritten, plan);
    assert_eq!(records, [kept_for(RELATIONSHIP)]);
}

#[test]
fn e5_3_2a_a_join_that_does_not_follow_its_relationship_keeps_the_deduplicate() {
    let state = state(&[customers_unique()]);
    let reversed = join(
        home(true),
        joined_scan("ORDERS", RELATIONSHIP, &["CUSTOMER_ID"]),
        RELATIONSHIP,
        ("CUSTOMER_ID", "ORDERS_TO_CUSTOMERS.CUSTOMER_ID"),
        RELATIONSHIP,
    );
    let plan = deduplicated(reversed);

    let (rewritten, records) = run(plan.clone(), &shop_model(), Some(&state));

    assert_eq!(rewritten, plan);
    let failed_condition = format!(
        "the Join of relationship {RELATIONSHIP} does not follow it from `from` to `to` (E5-3.2a)"
    );
    assert_eq!(
        records,
        [record(RewriteOutcome::Rejected { failed_condition })]
    );
}

#[test]
fn e5_3_8_a_plan_without_a_deduplicate_has_no_record() {
    let state = state(&[customers_unique()]);
    let plan = limited(one_join(false));

    let (rewritten, records) = run(plan.clone(), &shop_model(), Some(&state));

    assert_eq!(rewritten, plan);
    assert!(records.is_empty());
}
