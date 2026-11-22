//! Unit tests of the inner-join rewrite (E5-3.3, E5-3.3a, E5-3.8).

use menura_contracts::types::catalog::CatalogStateDocument;
use menura_contracts::types::common::LogicalType;
use menura_model::expression::{Expression, Identifier, Literal, Reference};
use menura_model::internal_model::{
    CheckKind, Field, InternalModel, NormalizedName, Relationship, TypedValue, ValueType,
};
use menura_representations::logical_plan::{
    JoinCondition, JoinKind, Operation, OutputSchema, PlanNode, SchemaColumn,
};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;
use serde_json::{Value, json};

use crate::test_support::{CURRENT_HASH, anyone, empty_query, shop_model, shop_schema};
use crate::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject, optimize};

const ORDERS_TO_CUSTOMERS: &str = "ORDERS_TO_CUSTOMERS";
const CUSTOMERS_TO_REGIONS: &str = "CUSTOMERS_TO_REGIONS";
const EVIDENCE: &str = "passing `orphan_keys` evidence for the relationship (CAT-7.3)";
const NOT_NULL: &str =
    "every `from_columns` column has `nullable: false` in the catalog schema (E5-3.3a (1))";
const FROM_SIDE: &str =
    "the `from` side is the home `Scan` or a dataset joined by an `inner` join (E5-3.3a (2))";
const JOINED_SIDE: &str = "the joined `Scan` reads every row of `to` for this user (E5-3.3a (3))";

/// The integer logical type.
fn integer() -> LogicalType {
    serde_json::from_str(r#"{"kind": "integer", "bits": 32, "signed": true}"#)
        .expect("a logical type")
}

/// The field `name`: a plain reference to its column, of type integer, NULL only when
/// `nullable`.
fn column_field(template: &Field, name: &str, nullable: bool) -> Field {
    Field {
        name: NormalizedName::new(name),
        expression: Some(Expression::Reference(Reference {
            qualifier: None,
            name: Identifier::regular(name).expect("a regular identifier"),
        })),
        typed: Some(TypedValue {
            value_type: ValueType::Logical(integer()),
            nullable,
        }),
        ..template.clone()
    }
}

/// The model: `ORDERS` related to `CUSTOMERS` by `ORDERS_TO_CUSTOMERS` (on `CUSTOMER_ID`),
/// and `CUSTOMERS` related to `REGIONS` by `CUSTOMERS_TO_REGIONS` (on `REGION_ID`), with the
/// `orphan_keys` checks of both recorded. `change` edits the `CUSTOMER_ID` field of `ORDERS`.
fn model_with(change: impl Fn(&mut Field)) -> InternalModel {
    let base = shop_model();
    let mut parts = base.parts().clone();
    let template = Field {
        name: NormalizedName::new("TEMPLATE"),
        expression: None,
        typed: None,
        is_time: None,
        descriptions: parts.datasets[0].descriptions.clone(),
        query_errors: Vec::new(),
        location: parts.datasets[0].location.clone(),
    };
    let mut regions = parts.datasets[0].clone();
    regions.name = NormalizedName::new("REGIONS");
    regions.source = "shop.main.regions".to_owned();
    regions.fields = vec![column_field(&template, "REGION_ID", false)];
    parts.datasets[0].fields = vec![
        column_field(&template, "CUSTOMER_ID", false),
        column_field(&template, "REGION_ID", false),
    ];
    let mut customer_id = column_field(&template, "CUSTOMER_ID", false);
    change(&mut customer_id);
    parts.datasets[1].fields = vec![customer_id];
    parts.datasets.push(regions);
    let regions_relationship = Relationship {
        name: NormalizedName::new(CUSTOMERS_TO_REGIONS),
        from: NormalizedName::new("CUSTOMERS"),
        to: NormalizedName::new("REGIONS"),
        from_columns: vec![NormalizedName::new("REGION_ID")],
        to_columns: vec![NormalizedName::new("REGION_ID")],
        ..parts.relationships[0].clone()
    };
    parts.relationships.insert(0, regions_relationship);
    let mut regions_check = parts
        .checks
        .iter()
        .find(|check| check.kind == CheckKind::OrphanKeys)
        .expect("the orphan_keys check")
        .clone();
    regions_check.check_id = format!("orphan_keys:{CUSTOMERS_TO_REGIONS}");
    regions_check.target = CUSTOMERS_TO_REGIONS.to_owned();
    parts.checks.push(regions_check);
    InternalModel::new(parts, base.hash().clone())
}

/// The model of `model_with` with every key field a non-null column.
fn model() -> InternalModel {
    model_with(|_| {})
}

/// The version token `snapshot`.
fn token(snapshot: &str) -> Value {
    json!({"origin": "warehouse:shop", "snapshot": snapshot})
}

/// A passing `orphan_keys` result for `target`, verified on the current versions of `from`
/// and `to`.
fn orphan_keys(target: &str, from: &str, to: &str) -> Value {
    json!({
        "target": target,
        "check": "orphan_keys",
        "definition_hash": CURRENT_HASH,
        "status": "passed",
        "verified": [
            {"source": format!("shop.main.{from}"), "version": token(from)},
            {"source": format!("shop.main.{to}"), "version": token(to)}
        ]
    })
}

/// The state with the three relations and passing `orphan_keys` evidence for both
/// relationships.
fn state() -> CatalogStateDocument {
    let relation = |name: &str| {
        json!({"source": format!("shop.main.{name}"), "row_count_estimate": 2,
            "version": token(name)})
    };
    serde_json::from_value(json!({
        "contract": "menura.catalog_state",
        "contract_version": "1.1.0",
        "document_id": token("state-1"),
        "schema_document_id": token("schema-1"),
        "relations": [relation("orders"), relation("customers"), relation("regions")],
        "checks": [
            orphan_keys(ORDERS_TO_CUSTOMERS, "orders", "customers"),
            orphan_keys(CUSTOMERS_TO_REGIONS, "customers", "regions")
        ]
    }))
    .expect("a well-formed catalog state")
}

/// The schema of the integer non-null columns `columns`.
fn schema(columns: &[String]) -> OutputSchema {
    OutputSchema::new(
        columns
            .iter()
            .map(|name| SchemaColumn {
                name: ColumnName::new(name),
                logical_type: integer(),
                nullable: false,
            })
            .collect(),
    )
}

/// The origin of every test node.
fn origin() -> Origin {
    Origin::Model(shop_model().parts().datasets[0].location.clone())
}

/// The `Scan` of `dataset`, producing `columns`, each under `prefix` when given.
fn scan(dataset: &str, columns: &[&str], prefix: Option<&str>) -> PlanNode {
    let columns: Vec<String> = columns
        .iter()
        .map(|column| format!("{}{column}", prefix.unwrap_or("")))
        .collect();
    PlanNode::new(
        Operation::Scan {
            dataset: NormalizedName::new(dataset),
            row_identity: None,
        },
        schema(&columns),
        origin(),
    )
}

/// `input` under a row-policy `Filter` whose predicate is `FALSE`.
fn filtered(input: PlanNode) -> PlanNode {
    let schema = input.schema().clone();
    PlanNode::new(
        Operation::Filter {
            input: Box::new(input),
            predicate: Expression::Literal(Literal::Boolean(false)),
        },
        schema,
        origin(),
    )
}

/// The `left` `Join` of `left` and `right` along `relationship`, on `left_column` equal to
/// the unprefixed `right_column` under `prefix`.
fn join(
    left: PlanNode,
    right: PlanNode,
    relationship: &str,
    (left_column, right_column): (&str, &str),
    prefix: &str,
) -> PlanNode {
    let columns = left
        .schema()
        .columns()
        .iter()
        .chain(right.schema().columns())
        .map(|column| column.name.as_str().to_owned())
        .collect::<Vec<_>>();
    PlanNode::new(
        Operation::Join {
            left: Box::new(left),
            right: Box::new(right),
            relationship: NormalizedName::new(relationship),
            kind: JoinKind::Left,
            on: vec![JoinCondition {
                left: ColumnName::new(left_column),
                right: ColumnName::new(&format!("{prefix}{right_column}")),
            }],
            right_prefix: ColumnPrefix::new(prefix),
        },
        schema(&columns),
        origin(),
    )
}

const CUSTOMERS_PREFIX: &str = "ORDERS_TO_CUSTOMERS.";
const REGIONS_PREFIX: &str = "CUSTOMERS_TO_REGIONS.";

/// `ORDERS` joined to `CUSTOMERS`, whose joined side is `customers`.
fn orders_to(customers: PlanNode) -> PlanNode {
    join(
        scan("ORDERS", &["CUSTOMER_ID"], None),
        customers,
        ORDERS_TO_CUSTOMERS,
        ("CUSTOMER_ID", "CUSTOMER_ID"),
        CUSTOMERS_PREFIX,
    )
}

/// The bare `Scan` of `CUSTOMERS` as joined from `ORDERS`.
fn customers() -> PlanNode {
    scan(
        "CUSTOMERS",
        &["CUSTOMER_ID", "REGION_ID"],
        Some(CUSTOMERS_PREFIX),
    )
}

/// `chain` joined further to `REGIONS` from the joined `CUSTOMERS`.
fn then_regions(chain: PlanNode) -> PlanNode {
    join(
        chain,
        scan("REGIONS", &["REGION_ID"], Some(REGIONS_PREFIX)),
        CUSTOMERS_TO_REGIONS,
        (&format!("{CUSTOMERS_PREFIX}REGION_ID"), "REGION_ID"),
        REGIONS_PREFIX,
    )
}

/// The plan and records of E5 for `plan`, with `model` and `state`.
fn run(
    plan: &PlanNode,
    model: &InternalModel,
    state: Option<&CatalogStateDocument>,
) -> (PlanNode, Vec<RewriteRecord>) {
    let optimized = optimize(
        plan,
        &empty_query(),
        model,
        &shop_schema(),
        state,
        &anyone(),
    );
    let records = optimized
        .records()
        .iter()
        .filter(|record| record.rule == RewriteRule::InnerJoins)
        .cloned()
        .collect();
    (optimized.into_plan(), records)
}

/// The kinds of the joins of `plan`, in path order.
fn kinds(plan: &PlanNode) -> Vec<JoinKind> {
    let mut kinds: Vec<JoinKind> = plan.inputs().into_iter().flat_map(kinds).collect();
    if let Operation::Join { kind, .. } = plan.operation() {
        kinds.push(*kind);
    }
    kinds
}

/// The record of the join at `steps`, applied or rejected with `failed`.
fn record(steps: Vec<usize>, failed: Option<&str>) -> RewriteRecord {
    RewriteRecord {
        rule: RewriteRule::InnerJoins,
        subject: RewriteSubject::Node(NodePath::new(steps)),
        outcome: failed.map_or(RewriteOutcome::Applied, |condition| {
            RewriteOutcome::Rejected {
                failed_condition: condition.to_owned(),
            }
        }),
    }
}

#[test]
fn e5_3_3_a_left_join_becomes_inner_when_every_condition_holds() {
    let (plan, records) = run(&orders_to(customers()), &model(), Some(&state()));

    assert_eq!(kinds(&plan), [JoinKind::Inner]);
    assert_eq!(records, [record(Vec::new(), None)]);
}

#[test]
fn e5_3_3_without_orphan_keys_evidence_the_join_stays_left() {
    let (plan, records) = run(&orders_to(customers()), &model(), None);

    assert_eq!(kinds(&plan), [JoinKind::Left]);
    assert_eq!(records, [record(Vec::new(), Some(EVIDENCE))]);
}

#[test]
fn e5_3_3a_a_nullable_from_column_keeps_the_join_left() {
    let model = model_with(|field| {
        if let Some(typed) = &mut field.typed {
            typed.nullable = true;
        }
    });

    let (plan, records) = run(&orders_to(customers()), &model, Some(&state()));

    assert_eq!(kinds(&plan), [JoinKind::Left]);
    assert_eq!(records, [record(Vec::new(), Some(NOT_NULL))]);
}

#[test]
fn e5_3_3a_a_from_column_that_is_not_a_catalog_column_keeps_the_join_left() {
    let model = model_with(|field| {
        field.expression = Some(Expression::Literal(Literal::Boolean(true)));
    });

    let (plan, records) = run(&orders_to(customers()), &model, Some(&state()));

    assert_eq!(kinds(&plan), [JoinKind::Left]);
    assert_eq!(records, [record(Vec::new(), Some(NOT_NULL))]);
}

#[test]
fn e5_3_3a_a_from_side_joined_by_a_left_join_keeps_the_join_left() {
    let plan = then_regions(orders_to(filtered(customers())));

    let (plan, records) = run(&plan, &model(), Some(&state()));

    assert_eq!(kinds(&plan), [JoinKind::Left, JoinKind::Left]);
    assert_eq!(
        records,
        [
            record(vec![0], Some(JOINED_SIDE)),
            record(Vec::new(), Some(FROM_SIDE)),
        ]
    );
}

#[test]
fn e5_3_3a_a_from_side_joined_by_an_inner_join_allows_inner() {
    let plan = then_regions(orders_to(customers()));

    let (plan, records) = run(&plan, &model(), Some(&state()));

    assert_eq!(kinds(&plan), [JoinKind::Inner, JoinKind::Inner]);
    assert_eq!(records, [record(vec![0], None), record(Vec::new(), None)]);
}

#[test]
fn e5_3_3a_a_filtered_joined_scan_keeps_the_join_left() {
    let (plan, records) = run(&orders_to(filtered(customers())), &model(), Some(&state()));

    assert_eq!(kinds(&plan), [JoinKind::Left]);
    assert_eq!(records, [record(Vec::new(), Some(JOINED_SIDE))]);
}

#[test]
fn e5_3_3a_a_joined_scan_of_another_dataset_keeps_the_join_left() {
    let other = scan("ORDERS", &["CUSTOMER_ID"], Some(CUSTOMERS_PREFIX));

    let (plan, records) = run(&orders_to(other), &model(), Some(&state()));

    assert_eq!(kinds(&plan), [JoinKind::Left]);
    assert_eq!(records, [record(Vec::new(), Some(JOINED_SIDE))]);
}

#[test]
fn e5_3_3_joins_below_other_nodes_are_rewritten_at_their_path() {
    let plan = filtered(orders_to(customers()));

    let (plan, records) = run(&plan, &model(), Some(&state()));

    assert_eq!(kinds(&plan), [JoinKind::Inner]);
    assert_eq!(records, [record(vec![0], None)]);
    assert!(matches!(plan.operation(), Operation::Filter { .. }));
}
