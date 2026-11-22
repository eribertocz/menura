//! Unit tests of the removal of unused joins (E5-3.1, E5-3.1a, E5-3.7, E5-3.8).

use menura_contracts::types::common::LogicalType;
use menura_contracts::types::query::{Direction, NullsOrder};
use menura_model::expression::{
    AggregateCall, BinaryOperator, Expression, Identifier, Literal, Reference, SimpleAggregate,
};
use menura_model::internal_model::{NormalizedName, SourceLocation};
use menura_representations::logical_plan::{
    AggregateComponent, JoinCondition, JoinKind, NamedExpression, Operation, OutputSchema,
    PlanNode, SchemaColumn, SortItem,
};
use menura_representations::names::{ColumnName, ColumnPrefix};
use menura_representations::origin::Origin;

use super::unused_joins::rewrite;
use crate::context::RewriteContext;
use crate::evidence::Evidence;
use crate::record::{NodePath, RewriteOutcome, RewriteRecord, RewriteRule, RewriteSubject};
use crate::test_support::{anyone, empty_query, shop_model, shop_schema};

/// The column of the synthesized home row identity.
const ROW_IDENTITY: &str = "ROW_IDENTITY";
/// The prefix of the columns of the `CUSTOMERS` join instance.
const CUSTOMERS: &str = "ORDERS_TO_CUSTOMERS.";
/// The prefix of the columns of the `REGIONS` join instance, reached through `CUSTOMERS`.
const REGIONS: &str = "ORDERS_TO_CUSTOMERS.CUSTOMERS_TO_REGIONS.";

/// The origin of every test node.
fn origin() -> Origin {
    Origin::Model(SourceLocation {
        pointer: "/semantic_model/0".to_owned(),
        line: 1,
        column: 1,
    })
}

/// A nullable integer column `name`.
fn column(name: &str) -> SchemaColumn {
    SchemaColumn {
        name: ColumnName::new(name),
        logical_type: serde_json::from_str::<LogicalType>(
            r#"{"kind": "integer", "bits": 32, "signed": true}"#,
        )
        .expect("a logical type"),
        nullable: true,
    }
}

/// The schema of the columns `names`, in order.
fn schema(names: &[String]) -> OutputSchema {
    OutputSchema::new(names.iter().map(|name| column(name)).collect())
}

/// The names of the columns of `node`, in order.
fn names(node: &PlanNode) -> Vec<String> {
    node.schema()
        .columns()
        .iter()
        .map(|column| column.name.as_str().to_owned())
        .collect()
}

/// The reference to the plan column `name`.
fn reference(name: &str) -> Expression {
    Expression::Reference(Reference {
        qualifier: None,
        name: Identifier::delimited(name).expect("a non-empty name"),
    })
}

/// The home `Scan` of `ORDERS`, with the row identity when `with_identity`.
fn home_scan(with_identity: bool) -> PlanNode {
    let mut columns = vec!["CUSTOMER_ID".to_owned(), "AMOUNT".to_owned()];
    if with_identity {
        columns.push(ROW_IDENTITY.to_owned());
    }
    PlanNode::new(
        Operation::Scan {
            dataset: NormalizedName::new("ORDERS"),
            row_identity: with_identity.then(|| ColumnName::new(ROW_IDENTITY)),
        },
        schema(&columns),
        origin(),
    )
}

/// The joined `Scan` of `dataset`, whose columns are `fields` under `prefix`.
fn joined_scan(dataset: &str, prefix: &str, fields: &[&str]) -> PlanNode {
    let columns: Vec<String> = fields
        .iter()
        .map(|field| format!("{prefix}{field}"))
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

/// `Join(left, right)` along `relationship`, on `left_column = right_column`.
fn join(
    left: PlanNode,
    right: PlanNode,
    relationship: &str,
    left_column: &str,
    right_column: &str,
    prefix: &str,
) -> PlanNode {
    let mut columns = names(&left);
    columns.extend(names(&right));
    PlanNode::new(
        Operation::Join {
            left: Box::new(left),
            right: Box::new(right),
            relationship: NormalizedName::new(relationship),
            kind: JoinKind::Left,
            on: vec![JoinCondition {
                left: ColumnName::new(left_column),
                right: ColumnName::new(right_column),
            }],
            right_prefix: ColumnPrefix::new(prefix),
        },
        schema(&columns),
        origin(),
    )
}

/// `home` joined to `CUSTOMERS`.
fn join_customers(home: PlanNode) -> PlanNode {
    join(
        home,
        joined_scan(
            "CUSTOMERS",
            CUSTOMERS,
            &["CUSTOMER_ID", "REGION_ID", "SEGMENT"],
        ),
        "ORDERS_TO_CUSTOMERS",
        "CUSTOMER_ID",
        &format!("{CUSTOMERS}CUSTOMER_ID"),
        CUSTOMERS,
    )
}

/// `customers`, a plan that joins `CUSTOMERS`, joined to `REGIONS` from `CUSTOMERS`.
fn join_regions(customers: PlanNode) -> PlanNode {
    join(
        customers,
        joined_scan("REGIONS", REGIONS, &["REGION_ID", "NAME"]),
        "CUSTOMERS_TO_REGIONS",
        &format!("{CUSTOMERS}REGION_ID"),
        &format!("{REGIONS}REGION_ID"),
        REGIONS,
    )
}

/// `Filter(input, predicate)`.
fn filter(input: PlanNode, predicate: Expression) -> PlanNode {
    let columns = names(&input);
    PlanNode::new(
        Operation::Filter {
            input: Box::new(input),
            predicate,
        },
        schema(&columns),
        origin(),
    )
}

/// `Deduplicate(input, group, identity)` with the row identity.
fn deduplicate(input: PlanNode, group: Vec<Expression>) -> PlanNode {
    let columns = names(&input);
    PlanNode::new(
        Operation::Deduplicate {
            input: Box::new(input),
            group,
            identity: vec![ColumnName::new(ROW_IDENTITY)],
        },
        schema(&columns),
        origin(),
    )
}

/// `Aggregate(input, keys, components)`: the keys are the `key` columns, named `KEY_n`.
fn aggregate(input: PlanNode, keys: &[&str], components: Vec<AggregateComponent>) -> PlanNode {
    let keys: Vec<NamedExpression> = keys
        .iter()
        .enumerate()
        .map(|(position, key)| NamedExpression {
            name: ColumnName::new(&format!("KEY_{position}")),
            expression: reference(key),
            derived: None,
        })
        .collect();
    let mut columns: Vec<String> = keys
        .iter()
        .map(|key| key.name.as_str().to_owned())
        .collect();
    columns.extend(
        components
            .iter()
            .map(|component| component.name.as_str().to_owned()),
    );
    PlanNode::new(
        Operation::Aggregate {
            input: Box::new(input),
            keys,
            components,
        },
        schema(&columns),
        origin(),
    )
}

/// `SUM(argument)` named `TOTAL`.
fn sum(argument: &str) -> AggregateComponent {
    AggregateComponent {
        name: ColumnName::new("TOTAL"),
        call: AggregateCall::Simple {
            function: SimpleAggregate::Sum,
            argument: Box::new(reference(argument)),
        },
        filter: None,
        recomposition: None,
    }
}

/// The metric branch `Aggregate(Deduplicate(body), keys, SUM(AMOUNT))`.
fn metric_branch(body: PlanNode, keys: &[&str]) -> PlanNode {
    let group = keys.iter().map(|key| reference(key)).collect();
    aggregate(deduplicate(body, group), keys, vec![sum("AMOUNT")])
}

/// The metric branch over `home` without joins: what an applied rewrite leaves.
fn branch_without_joins(home: PlanNode, keys: &[&str]) -> PlanNode {
    aggregate(home, keys, vec![sum("AMOUNT")])
}

/// Runs the rewrite on `plan` with no catalog state, returning the plan and its records.
fn run(plan: PlanNode) -> (PlanNode, Vec<RewriteRecord>) {
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

/// The record of E5-3.1 for the node at `steps`, applied.
fn applied(steps: &[usize]) -> RewriteRecord {
    RewriteRecord {
        rule: RewriteRule::RemoveUnusedJoins,
        subject: RewriteSubject::Node(NodePath::new(steps.to_vec())),
        outcome: RewriteOutcome::Applied,
    }
}

/// Asserts that the rewrite keeps `plan`, its one `Join` being at `steps`, and records
/// nothing: a used join is not considered (E5-3.8).
fn assert_kept(plan: &PlanNode, steps: &[usize]) {
    let (rewritten, records) = run(plan.clone());

    assert_eq!(&rewritten, plan);
    assert!(records.is_empty(), "the join at {steps:?} is recorded");
}

#[test]
fn e5_3_1_one_unused_join_is_removed_and_the_used_one_is_kept() {
    let customers = join_customers(home_scan(true));
    let plan = metric_branch(
        join(
            customers,
            joined_scan("PRODUCTS", "ORDERS_TO_PRODUCTS.", &["CUSTOMER_ID"]),
            "ORDERS_TO_PRODUCTS",
            "CUSTOMER_ID",
            "ORDERS_TO_PRODUCTS.CUSTOMER_ID",
            "ORDERS_TO_PRODUCTS.",
        ),
        &["ORDERS_TO_PRODUCTS.CUSTOMER_ID"],
    );
    let expected = metric_branch(
        join(
            home_scan(true),
            joined_scan("PRODUCTS", "ORDERS_TO_PRODUCTS.", &["CUSTOMER_ID"]),
            "ORDERS_TO_PRODUCTS",
            "CUSTOMER_ID",
            "ORDERS_TO_PRODUCTS.CUSTOMER_ID",
            "ORDERS_TO_PRODUCTS.",
        ),
        &["ORDERS_TO_PRODUCTS.CUSTOMER_ID"],
    );

    let (rewritten, records) = run(plan);

    assert_eq!(rewritten, expected);
    assert_eq!(records, vec![applied(&[0, 0, 0])]);
}

#[test]
fn e5_3_1a_a_chain_of_two_unused_joins_is_removed_whole() {
    let plan = metric_branch(join_regions(join_customers(home_scan(true))), &["AMOUNT"]);

    let (rewritten, records) = run(plan);

    assert_eq!(
        rewritten,
        branch_without_joins(home_scan(false), &["AMOUNT"])
    );
    assert_eq!(records, vec![applied(&[0, 0]), applied(&[0, 0, 0])]);
}

#[test]
fn e5_3_1a_a_join_read_only_by_the_condition_of_another_join_is_kept() {
    let key = format!("{REGIONS}NAME");
    let plan = metric_branch(join_regions(join_customers(home_scan(true))), &[&key]);
    let (rewritten, records) = run(plan.clone());

    assert_eq!(rewritten, plan);
    assert!(records.is_empty());
}

#[test]
fn e5_3_1a_a_join_read_by_a_query_filter_is_kept() {
    let predicate = Expression::Binary {
        operator: BinaryOperator::Equal,
        left: Box::new(reference(&format!("{CUSTOMERS}SEGMENT"))),
        right: Box::new(Expression::Literal(Literal::Text("A".to_owned()))),
    };
    let plan = metric_branch(
        filter(join_customers(home_scan(true)), predicate),
        &["AMOUNT"],
    );

    assert_kept(&plan, &[0, 0, 0]);
}

#[test]
fn e5_3_1a_a_join_read_only_by_the_date_range_predicate_is_kept() {
    let predicate = Expression::Between {
        operand: Box::new(reference(&format!("{CUSTOMERS}SEGMENT"))),
        low: Box::new(Expression::Literal(Literal::Text("A".to_owned()))),
        high: Box::new(Expression::Literal(Literal::Text("Z".to_owned()))),
        negated: false,
    };
    let plan = metric_branch(
        filter(join_customers(home_scan(true)), predicate),
        &["AMOUNT"],
    );

    assert_kept(&plan, &[0, 0, 0]);
}

#[test]
fn e5_3_1a_a_join_read_by_an_aggregate_key_is_kept() {
    let key = format!("{CUSTOMERS}SEGMENT");
    let plan = metric_branch(join_customers(home_scan(true)), &[&key]);

    assert_kept(&plan, &[0, 0]);
}

#[test]
fn e5_3_1a_a_join_read_by_a_component_is_kept() {
    let component = sum(&format!("{CUSTOMERS}SEGMENT"));
    let plan = aggregate(
        deduplicate(join_customers(home_scan(true)), Vec::new()),
        &[],
        vec![component],
    );

    assert_kept(&plan, &[0, 0]);
}

#[test]
fn e5_3_1a_a_join_read_by_an_order_by_column_is_kept() {
    let branch = metric_branch(join_customers(home_scan(true)), &["AMOUNT"]);
    let columns = names(&branch);
    let plan = PlanNode::new(
        Operation::Sort {
            input: Box::new(branch),
            items: vec![SortItem {
                column: ColumnName::new(&format!("{CUSTOMERS}SEGMENT")),
                direction: Direction::Asc,
                nulls: NullsOrder::Last,
            }],
        },
        schema(&columns),
        origin(),
    );

    assert_kept(&plan, &[0, 0, 0]);
}

#[test]
fn e5_3_1a_the_last_join_goes_with_its_deduplicate_and_the_row_identity() {
    let plan = metric_branch(join_customers(home_scan(true)), &["AMOUNT"]);

    let (rewritten, records) = run(plan);

    assert_eq!(
        rewritten,
        branch_without_joins(home_scan(false), &["AMOUNT"])
    );
    assert!(rewritten.is_simple_plan());
    assert_eq!(records, vec![applied(&[0, 0])]);
}

#[test]
fn e5_3_7_the_semi_joins_and_filters_of_row_policies_are_kept() {
    let related = joined_scan("CUSTOMERS", "", &["CUSTOMER_ID"]);
    let home = |with_identity: bool| {
        let scan = home_scan(with_identity);
        let columns = names(&scan);
        let policy = filter(scan, Expression::Literal(Literal::Boolean(true)));
        PlanNode::new(
            Operation::SemiJoin {
                input: Box::new(policy),
                relationship: NormalizedName::new("ORDERS_TO_CUSTOMERS"),
                on: vec![JoinCondition {
                    left: ColumnName::new("CUSTOMER_ID"),
                    right: ColumnName::new("CUSTOMER_ID"),
                }],
                related_all: Box::new(related.clone()),
                related_visible: Box::new(related.clone()),
            },
            schema(&columns),
            origin(),
        )
    };
    let plan = metric_branch(join_customers(home(true)), &["AMOUNT"]);

    let (rewritten, records) = run(plan);

    assert_eq!(rewritten, branch_without_joins(home(false), &["AMOUNT"]));
    assert_eq!(records, vec![applied(&[0, 0])]);
}

#[test]
fn e5_3_1_a_plan_without_joins_is_unchanged_and_unrecorded() {
    let plan = branch_without_joins(home_scan(false), &["AMOUNT"]);

    let (rewritten, records) = run(plan.clone());

    assert_eq!(rewritten, plan);
    assert!(records.is_empty());
}
