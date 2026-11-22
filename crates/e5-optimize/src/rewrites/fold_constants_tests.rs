//! Unit tests of E5-3.6, fold constants.

use menura_model::expression::{
    AggregateCall, BinaryOperator, Expression, Identifier, Literal, Reference,
};
use menura_model::internal_model::{NormalizedName, SourceLocation};
use menura_representations::logical_plan::{
    AggregateComponent, JoinCondition, JoinKind, NamedExpression, Operation, PlanNode, StitchColumn,
};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::{Origin, QueryLocation};

use crate::test_support::{
    RELATIONSHIP, anyone, empty_query, filtered_scan, shop_model, shop_schema,
};
use crate::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject, optimize};

fn model_origin() -> Origin {
    Origin::Model(SourceLocation {
        pointer: "/semantic_model/0/datasets/1".to_owned(),
        line: 1,
        column: 1,
    })
}

/// `node` with the schema of `filtered_scan` and model origin.
fn node(operation: Operation) -> PlanNode {
    PlanNode::new(operation, filtered_scan().schema().clone(), model_origin())
}

fn boolean(value: bool) -> Expression {
    Expression::Literal(Literal::Boolean(value))
}

/// `G AND R` (E4-6.2) with the grant `granted`.
fn row_filter(granted: bool) -> Expression {
    Expression::Binary {
        operator: BinaryOperator::And,
        left: Box::new(boolean(granted)),
        right: Box::new(boolean(true)),
    }
}

/// The visible rows of `dataset` under its row-policy `Filter`, granted or not.
fn policy(dataset: &str, granted: bool) -> PlanNode {
    let scan = node(Operation::Scan {
        dataset: NormalizedName::new(dataset),
        row_identity: None,
    });
    node(Operation::Filter {
        input: Box::new(scan),
        predicate: row_filter(granted),
    })
}

fn key_condition() -> Vec<JoinCondition> {
    vec![JoinCondition {
        left: ColumnName::new("CUSTOMER_ID"),
        right: ColumnName::new("CUSTOMER_ID"),
    }]
}

/// The propagation `SemiJoin` of `input` towards `CUSTOMERS`.
fn semi_join(input: PlanNode) -> PlanNode {
    node(Operation::SemiJoin {
        input: Box::new(input),
        relationship: NormalizedName::new(RELATIONSHIP),
        on: key_condition(),
        related_all: Box::new(policy("CUSTOMERS", true)),
        related_visible: Box::new(policy("CUSTOMERS", true)),
    })
}

fn join(left: PlanNode, right: PlanNode) -> PlanNode {
    node(Operation::Join {
        left: Box::new(left),
        right: Box::new(right),
        relationship: NormalizedName::new(RELATIONSHIP),
        kind: JoinKind::Left,
        on: key_condition(),
        right_prefix: ColumnPrefix::new("CUSTOMERS"),
    })
}

fn deduplicate(input: PlanNode) -> PlanNode {
    node(Operation::Deduplicate {
        input: Box::new(input),
        group: Vec::new(),
        identity: vec![ColumnName::new("menura.row")],
    })
}

/// An `Aggregate` of `COUNT(*)` over `input`, grouped by `CUSTOMER_ID` when `keyed`.
fn aggregate(input: PlanNode, keyed: bool) -> PlanNode {
    let key = NamedExpression {
        name: ColumnName::new("CUSTOMER_ID"),
        expression: Expression::Reference(Reference {
            qualifier: None,
            name: Identifier::regular("CUSTOMER_ID").expect("an identifier"),
        }),
        derived: None,
    };
    node(Operation::Aggregate {
        input: Box::new(input),
        keys: if keyed { vec![key] } else { Vec::new() },
        components: vec![AggregateComponent {
            name: ColumnName::new("ROWS"),
            call: AggregateCall::CountRows,
            filter: None,
            recomposition: None,
        }],
    })
}

/// A branch over `ORDERS` joined to `CUSTOMERS`, with the home grant `home` and the joined
/// grant `joined`: its home rows pass a propagation `SemiJoin` and are deduplicated.
fn branch(home: bool, joined: bool, keyed: bool) -> PlanNode {
    let rows = join(
        semi_join(policy("ORDERS", home)),
        policy("CUSTOMERS", joined),
    );
    aggregate(deduplicate(rows), keyed)
}

/// The folded branch: the joins over the home row-policy `Filter`, without its `SemiJoin`
/// and without the `Deduplicate`.
fn folded_branch(keyed: bool) -> PlanNode {
    aggregate(
        join(policy("ORDERS", false), policy("CUSTOMERS", true)),
        keyed,
    )
}

fn project(input: PlanNode) -> PlanNode {
    node(Operation::Project {
        input: Box::new(input),
        expressions: Vec::new(),
    })
}

fn stitch(branches: Vec<PlanNode>) -> PlanNode {
    node(Operation::Stitch {
        branches,
        keys: Vec::new(),
        columns: vec![StitchColumn {
            name: ColumnName::new("ROWS"),
            branch: 0,
            empty_value: None,
        }],
    })
}

fn applied(steps: Vec<usize>) -> RewriteRecord {
    RewriteRecord {
        rule: RewriteRule::FoldConstants,
        subject: RewriteSubject::Node(NodePath::new(steps)),
        outcome: RewriteOutcome::Applied,
    }
}

fn run(plan: &PlanNode) -> (PlanNode, Vec<RewriteRecord>) {
    let optimized = optimize(
        plan,
        &empty_query(),
        &shop_model(),
        &shop_schema(),
        None,
        &anyone(),
    );
    (optimized.plan().clone(), optimized.records().to_vec())
}

#[test]
fn e5_3_6_a_single_branch_with_keys_and_no_home_grant_is_folded() {
    let (plan, records) = run(&project(branch(false, true, true)));

    assert_eq!(plan, project(folded_branch(true)));
    assert_eq!(records, [applied(vec![0, 0, 0, 0, 0])]);
}

#[test]
fn e5_3_6_a_single_branch_without_keys_and_no_home_grant_is_folded() {
    let (plan, records) = run(&branch(false, true, false));

    assert_eq!(plan, folded_branch(false));
    assert_eq!(records, [applied(vec![0, 0, 0, 0])]);
}

#[test]
fn e5_3_6_only_the_stitched_branch_with_no_home_grant_is_folded() {
    let granted = branch(true, true, true);

    let (plan, records) = run(&stitch(vec![granted.clone(), branch(false, true, true)]));

    assert_eq!(plan, stitch(vec![granted, folded_branch(true)]));
    assert_eq!(records, [applied(vec![1, 0, 0, 0, 0])]);
}

#[test]
fn e5_3_6_a_joined_dataset_with_no_grant_is_not_folded() {
    let plan = branch(true, false, true);

    let (rewritten, records) = run(&plan);

    assert_eq!(rewritten, plan);
    assert!(records.is_empty());
}

#[test]
fn e5_3_6_a_false_query_filter_is_not_a_row_policy() {
    let scan = node(Operation::Scan {
        dataset: NormalizedName::new("ORDERS"),
        row_identity: None,
    });
    let query_filter = PlanNode::new(
        Operation::Filter {
            input: Box::new(scan),
            predicate: boolean(false),
        },
        filtered_scan().schema().clone(),
        Origin::Query(QueryLocation::new("/query/filters/0", None)),
    );
    let plan = aggregate(query_filter, true);

    let (rewritten, records) = run(&plan);

    assert_eq!(rewritten, plan);
    assert!(records.is_empty());
}
