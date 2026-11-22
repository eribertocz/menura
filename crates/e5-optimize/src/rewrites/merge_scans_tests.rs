//! Unit tests of the merge of identical branches (E5-3.4, E5-3.4a).

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::expression::{
    AggregateCall, DateValue, Expression, Identifier, Literal, Reference,
};
use menura_model::internal_model::{NormalizedName, SourceLocation};
use menura_representations::logical_plan::{
    AggregateComponent, JoinCondition, JoinKind, Membership, NamedExpression, Operation,
    OutputSchema, PlanNode, PopulationRule, SchemaColumn, StitchColumn,
};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;

use super::rewrite;
use crate::context::RewriteContext;
use crate::evidence::Evidence;
use crate::record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};
use crate::test_support::{RELATIONSHIP, anyone, empty_query, shop_model, shop_schema};

fn origin() -> Origin {
    Origin::Model(SourceLocation {
        pointer: "/semantic_model/0/datasets/1".to_owned(),
        line: 1,
        column: 1,
    })
}

fn integer() -> LogicalType {
    serde_json::from_str(r#"{"kind": "integer", "bits": 64, "signed": true}"#)
        .expect("a logical type")
}

fn column(name: &str) -> SchemaColumn {
    SchemaColumn {
        name: ColumnName::new(name),
        logical_type: integer(),
        nullable: true,
    }
}

fn schema(names: &[&str]) -> OutputSchema {
    OutputSchema::new(names.iter().map(|name| column(name)).collect())
}

fn node(operation: Operation, names: &[&str]) -> PlanNode {
    PlanNode::new(operation, schema(names), origin())
}

fn reference(name: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::delimited(name).expect("a name"),
    })
}

fn scan(dataset: &str) -> PlanNode {
    let operation = Operation::Scan {
        dataset: NormalizedName::new(dataset),
        row_identity: None,
    };
    node(operation, &["CUSTOMER_ID", "REGION"])
}

fn filter(input: PlanNode, value: bool) -> PlanNode {
    let operation = Operation::Filter {
        input: Box::new(input),
        predicate: Expression::Literal(Literal::Boolean(value)),
    };
    node(operation, &["CUSTOMER_ID", "REGION"])
}

fn join(left: PlanNode) -> PlanNode {
    let operation = Operation::Join {
        left: Box::new(left),
        right: Box::new(scan("CUSTOMERS")),
        relationship: NormalizedName::new(RELATIONSHIP),
        kind: JoinKind::Left,
        on: vec![JoinCondition {
            left: ColumnName::new("CUSTOMER_ID"),
            right: ColumnName::new("C.CUSTOMER_ID"),
        }],
        right_prefix: ColumnPrefix::new("C."),
    };
    node(operation, &["CUSTOMER_ID", "REGION"])
}

fn population() -> PlanNode {
    let periods = Operation::Periods {
        from: DateValue::new(2026, 1, 1).expect("a date"),
        to: DateValue::new(2026, 3, 31).expect("a date"),
        grain: Grain::Month,
    };
    let operation = Operation::Population {
        input: Box::new(scan("ORDERS")),
        dataset: NormalizedName::new("ORDERS"),
        population: NormalizedName::new("CUSTOMERS_SEEN"),
        grain: Grain::Month,
        periods: Box::new(node(periods, &["PERIOD"])),
        period: ColumnName::new("PERIOD"),
        rule: PopulationRule {
            entity: vec![ColumnName::new("CUSTOMER_ID")],
            membership: Membership::Periodic {
                period: ColumnName::new("REGION"),
            },
            required_when: None,
        },
    };
    node(operation, &["CUSTOMER_ID", "PERIOD"])
}

/// The `Aggregate` of `input` by `key` (a column of `input`) with one `COUNT(*)` per name of
/// `components`.
fn aggregate(input: PlanNode, key: &str, components: &[&str]) -> PlanNode {
    let mut names = vec!["dimension_1"];
    names.extend_from_slice(components);
    let operation = Operation::Aggregate {
        input: Box::new(input),
        keys: vec![NamedExpression {
            name: ColumnName::new("dimension_1"),
            expression: reference(key),
            derived: None,
        }],
        components: components
            .iter()
            .map(|name| AggregateComponent {
                name: ColumnName::new(name),
                call: AggregateCall::CountRows,
                filter: None,
                recomposition: None,
            })
            .collect(),
    };
    node(operation, &names)
}

fn branch(input: PlanNode, components: &[&str]) -> PlanNode {
    aggregate(input, "REGION", components)
}

fn components(branch: &PlanNode) -> Vec<&str> {
    match branch.operation() {
        Operation::Aggregate { components, .. } => components
            .iter()
            .map(|component| component.name.as_str())
            .collect(),
        other => panic!("not an aggregate: {other:?}"),
    }
}

/// `Project(Stitch(branches))`, as E4 builds it.
fn stitched(branches: Vec<PlanNode>) -> PlanNode {
    let mut names = vec!["dimension_1"];
    let mut columns = Vec::new();
    let all: Vec<Vec<String>> = branches
        .iter()
        .map(|branch| components(branch).into_iter().map(str::to_owned).collect())
        .collect();
    for (position, branch_components) in all.iter().enumerate() {
        for name in branch_components {
            names.push(name);
            columns.push(StitchColumn {
                name: ColumnName::new(name),
                branch: position,
                empty_value: None,
            });
        }
    }
    let stitch = node(
        Operation::Stitch {
            branches,
            keys: vec![ColumnName::new("dimension_1")],
            columns,
        },
        &names,
    );
    project(stitch)
}

fn project(input: PlanNode) -> PlanNode {
    let operation = Operation::Project {
        input: Box::new(input),
        expressions: vec![NamedExpression {
            name: ColumnName::new("region"),
            expression: reference("dimension_1"),
            derived: None,
        }],
    };
    node(operation, &["region"])
}

/// Runs the rewrite on `plan`.
fn merged(plan: PlanNode) -> (PlanNode, Vec<RewriteRecord>) {
    let model = shop_model();
    let schema = shop_schema();
    let query = empty_query();
    let security = anyone();
    let context = RewriteContext {
        query: &query,
        model: &model,
        schema: &schema,
        evidence: Evidence::new(&model, None),
        security: &security,
    };
    let mut records = Vec::new();
    let plan = rewrite(plan, &context, &mut records);
    (plan, records)
}

fn below_project(plan: &PlanNode) -> &PlanNode {
    match plan.operation() {
        Operation::Project { input, .. } => input,
        other => panic!("not a project: {other:?}"),
    }
}

fn record(branch: usize, outcome: RewriteOutcome) -> RewriteRecord {
    RewriteRecord {
        rule: RewriteRule::MergeScans,
        subject: RewriteSubject::Node(NodePath::new(vec![0, branch])),
        outcome,
    }
}

fn rejected(branch: usize, failed_condition: &str) -> RewriteRecord {
    record(
        branch,
        RewriteOutcome::Rejected {
            failed_condition: failed_condition.to_owned(),
        },
    )
}

const NOT_IDENTICAL: &str = "no earlier branch is identical except for its components (E5-3.4a)";
const SEMI_ADDITIVE: &str = "the branch has a Population, Snapshot or Periods node (E5-3.4a)";

#[test]
fn e5_3_4_two_identical_branches_become_one_aggregate_without_stitch() {
    let plan = stitched(vec![
        branch(filter(scan("ORDERS"), true), &["metric_1_component_1"]),
        branch(filter(scan("ORDERS"), true), &["metric_2_component_1"]),
    ]);

    let (plan, records) = merged(plan);

    let expected = project(branch(
        filter(scan("ORDERS"), true),
        &["metric_1_component_1", "metric_2_component_1"],
    ));
    assert_eq!(plan, expected);
    assert_eq!(records, [record(1, RewriteOutcome::Applied)]);
}

#[test]
fn e5_3_4a_branches_that_differ_in_a_query_filter_are_kept_apart() {
    let plan = stitched(vec![
        branch(filter(scan("ORDERS"), true), &["metric_1_component_1"]),
        branch(filter(scan("ORDERS"), false), &["metric_2_component_1"]),
    ]);

    let (rewritten, records) = merged(plan.clone());

    assert_eq!(rewritten, plan);
    assert_eq!(records, [rejected(1, NOT_IDENTICAL)]);
}

#[test]
fn e5_3_4a_branches_that_differ_in_a_join_are_kept_apart() {
    let plan = stitched(vec![
        branch(scan("ORDERS"), &["metric_1_component_1"]),
        branch(join(scan("ORDERS")), &["metric_2_component_1"]),
    ]);

    let (rewritten, records) = merged(plan.clone());

    assert_eq!(rewritten, plan);
    assert_eq!(records, [rejected(1, NOT_IDENTICAL)]);
}

#[test]
fn e5_3_4a_branches_that_differ_in_a_policy_node_are_kept_apart() {
    let plan = stitched(vec![
        branch(
            join(filter(scan("ORDERS"), true)),
            &["metric_1_component_1"],
        ),
        branch(join(scan("ORDERS")), &["metric_2_component_1"]),
    ]);

    let (rewritten, records) = merged(plan.clone());

    assert_eq!(rewritten, plan);
    assert_eq!(records, [rejected(1, NOT_IDENTICAL)]);
}

#[test]
fn e5_3_4a_branches_that_differ_in_their_keys_are_kept_apart() {
    let plan = stitched(vec![
        aggregate(scan("ORDERS"), "REGION", &["metric_1_component_1"]),
        aggregate(scan("ORDERS"), "CUSTOMER_ID", &["metric_2_component_1"]),
    ]);

    let (rewritten, records) = merged(plan.clone());

    assert_eq!(rewritten, plan);
    assert_eq!(records, [rejected(1, NOT_IDENTICAL)]);
}

/// The name and branch position of each `Stitch` column.
fn branch_positions(columns: &[StitchColumn]) -> Vec<(&str, usize)> {
    columns
        .iter()
        .map(|column| (column.name.as_str(), column.branch))
        .collect()
}

#[test]
fn e5_3_4a_two_of_three_branches_merge_at_the_position_of_the_first() {
    let plan = stitched(vec![
        branch(scan("ORDERS"), &["metric_1_component_1"]),
        branch(join(scan("ORDERS")), &["metric_2_component_1"]),
        branch(
            scan("ORDERS"),
            &["metric_3_component_1", "metric_3_component_2"],
        ),
    ]);
    let original = plan.clone();

    let (plan, records) = merged(plan);

    let Operation::Stitch {
        branches, columns, ..
    } = below_project(&plan).operation()
    else {
        panic!("not a stitch: {plan:?}");
    };
    let expected_branches = [
        branch(
            scan("ORDERS"),
            &[
                "metric_1_component_1",
                "metric_3_component_1",
                "metric_3_component_2",
            ],
        ),
        branch(join(scan("ORDERS")), &["metric_2_component_1"]),
    ];
    assert_eq!(branches, &expected_branches);
    assert_eq!(
        branch_positions(columns),
        [
            ("metric_1_component_1", 0),
            ("metric_2_component_1", 1),
            ("metric_3_component_1", 0),
            ("metric_3_component_2", 0),
        ]
    );
    assert_eq!(
        below_project(&plan).schema(),
        below_project(&original).schema()
    );
    assert_eq!(
        records,
        [
            rejected(1, NOT_IDENTICAL),
            record(2, RewriteOutcome::Applied)
        ]
    );
}

#[test]
fn e5_3_4a_a_branch_with_a_population_or_periods_node_is_never_merged() {
    let plan = stitched(vec![
        branch(population(), &["metric_1_component_1"]),
        branch(population(), &["metric_2_component_1"]),
    ]);

    let (rewritten, records) = merged(plan.clone());

    assert_eq!(rewritten, plan);
    assert_eq!(records, [rejected(1, SEMI_ADDITIVE)]);
}

#[test]
fn e5_3_4_a_plan_without_stitch_is_unchanged_and_records_nothing() {
    let plan = project(branch(scan("ORDERS"), &["metric_1_component_1"]));

    let (rewritten, records) = merged(plan.clone());

    assert_eq!(rewritten, plan);
    assert!(records.is_empty());
}

/// The home `Scan` of `ORDERS` that projects `columns`, under a row-policy `Filter` and a
/// `Join` whose right columns follow.
fn projecting(columns: &[&str]) -> PlanNode {
    let operation = Operation::Scan {
        dataset: NormalizedName::new("ORDERS"),
        row_identity: None,
    };
    let scan = node(operation, columns);
    let filter = node(
        Operation::Filter {
            input: Box::new(scan),
            predicate: Expression::Literal(Literal::Boolean(true)),
        },
        columns,
    );
    let joined = join(filter);
    let mut names = columns.to_vec();
    names.extend(["C.CUSTOMER_ID", "C.REGION"]);
    PlanNode::new(joined.operation().clone(), schema(&names), origin())
}

#[test]
fn e5_3_4a_branches_whose_home_scans_project_other_columns_merge_with_their_union() {
    let plan = stitched(vec![
        branch(
            projecting(&["CUSTOMER_ID", "REGION"]),
            &["metric_1_component_1"],
        ),
        branch(projecting(&["REGION", "AMOUNT"]), &["metric_2_component_1"]),
    ]);

    let (plan, records) = merged(plan);

    let expected = project(branch(
        projecting(&["CUSTOMER_ID", "REGION", "AMOUNT"]),
        &["metric_1_component_1", "metric_2_component_1"],
    ));
    assert_eq!(plan, expected);
    assert_eq!(records, [record(1, RewriteOutcome::Applied)]);
}

#[test]
fn e5_3_4a_the_merged_home_scan_lists_fields_in_field_order_then_sources_then_identity() {
    let fields = [
        NormalizedName::new("CUSTOMER_ID"),
        NormalizedName::new("REGION"),
        NormalizedName::new("AMOUNT"),
    ];
    let identity = ColumnName::new("menura.row_identity");

    let columns = super::rows::scan_columns(
        &schema(&["AMOUNT", "menura.paid", "menura.row_identity"]),
        &schema(&["CUSTOMER_ID", "menura.due", "menura.row_identity"]),
        &fields,
        Some(&identity),
    );

    let names: Vec<&str> = columns
        .columns()
        .iter()
        .map(|column| column.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "CUSTOMER_ID",
            "AMOUNT",
            "menura.paid",
            "menura.due",
            "menura.row_identity"
        ]
    );
}
