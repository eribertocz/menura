//! The plan of a semi-additive branch: a `Snapshot` of its rows, then the recomposition of its
//! components across entities (E4-3.13, E4-7, ADD-4).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use std::num::NonZeroU64;

use menura_contracts::types::common::Grain;
use menura_e4_plan::PlanError;
use menura_model::expression::{AggregateCall, Expression, SimpleAggregate};
use menura_model::internal_model::{
    DeclaredAdditivity, InternalModel, Metric, MissingHandling, NullHandling, Selection,
    SemiAdditive, SnapshotAggregation, StaleHandling, UnexpectedHandling,
};
use menura_representations::bound_query::{BoundDimension, BoundDimensionKind, BoundQueryParts};
use menura_representations::logical_plan::{
    Operation, PlanNode, SnapshotMissing, SnapshotOver, SnapshotRule, TimeSnapshot, UnitKey,
};
use menura_representations::names::ColumnName;
use menura_representations::paths::{Branch, BranchKind, Branches, ComponentKey, JoinTree};
use support::plan::{column_reference, kind, kinds, node};
use support::population::{active, subscriptions};
use support::{
    anyone, at, build_plan, date, date_range, dimension, field_path, metric, metric_reference,
    metrics, model_with, name, orders, query,
};

fn declaration(missing: MissingHandling) -> SemiAdditive {
    SemiAdditive {
        over: field_path("ORDER_DATE"),
        entity: vec![field_path("REGION"), field_path("TEAM")],
        aggregation: SnapshotAggregation::Last(Selection::GlobalPeriod),
        nulls: NullHandling::AsMissing,
        snapshot_grain: Some(Grain::Month),
        population: Some(name("ACTIVE")),
        missing: Some(missing),
        unexpected: Some(UnexpectedHandling::Exclude),
    }
}

fn balance(declared: SemiAdditive) -> Metric {
    Metric {
        declared_additivity: Some(DeclaredAdditivity::SemiAdditive(declared)),
        ..metric(
            "BALANCE",
            "SUM(orders.amount) + COUNT(*)",
            support::decimal_amount(),
        )
    }
}

fn model(declared: SemiAdditive) -> InternalModel {
    let mut all = metrics();
    all.push(balance(declared));
    all.sort_by(|left, right| left.name.cmp(&right.name));
    model_with(
        vec![
            orders(Vec::new()),
            subscriptions(vec![active(None)], Vec::new()),
        ],
        all,
    )
}

fn branch(kind: BranchKind, metric: usize, components: usize) -> Branch {
    Branch {
        home: name("ORDERS"),
        kind,
        components: (0..components)
            .map(|component| ComponentKey { metric, component })
            .collect(),
        join_tree: JoinTree::default(),
    }
}

fn balance_branch(metric: usize) -> Branch {
    branch(
        BranchKind::SemiAdditive {
            metric: name("BALANCE"),
        },
        metric,
        2,
    )
}

fn plan(
    declared: SemiAdditive,
    dimensions: Vec<BoundDimension>,
    parts: BoundQueryParts,
) -> Result<PlanNode, PlanError> {
    let branches = Branches::new(vec![balance_branch(0)]).expect("one branch");
    let query = query(dimensions, vec![metric_reference("BALANCE", 0)], parts);
    build_plan(&branches, &query, &anyone(), &model(declared))
}

fn snapshot_rule(plan: &PlanNode) -> &SnapshotRule {
    match node(plan, "Snapshot").operation() {
        Operation::Snapshot { rule, .. } => rule,
        other => panic!("not a Snapshot: {other:?}"),
    }
}

fn time(rule: &SnapshotRule) -> &TimeSnapshot {
    match &rule.over {
        SnapshotOver::Time(time) => time,
        SnapshotOver::Field(_) => panic!("a time over"),
    }
}

fn month() -> Vec<BoundDimension> {
    vec![dimension("ORDER_DATE", Some(Grain::Month), 0)]
}

#[test]
fn e4_3_13_a_semi_additive_branch_aggregates_a_snapshot_of_its_rows() {
    let plan = plan(
        declaration(MissingHandling::Exclude),
        month(),
        BoundQueryParts::default(),
    )
    .expect("a plan");

    assert_eq!(kinds(&plan), ["Project", "Aggregate", "Snapshot", "Scan"]);
    let rule = snapshot_rule(&plan);
    assert_eq!(rule.metric, name("BALANCE"));
    let entity: Vec<&str> = rule
        .entity
        .iter()
        .map(|named| named.name.as_str())
        .collect();
    assert_eq!(entity, ["entity_1", "entity_2"]);
    assert_eq!(rule.entity[0].expression, column_reference("REGION"));
    assert_eq!(rule.components.len(), 2);
    let time = time(rule);
    assert_eq!(time.grain, Grain::Month);
    assert_eq!(kind(&time.expected), "Population");
    assert_eq!(time.missing, SnapshotMissing::Exclude);
    assert_eq!(time.unexpected, UnexpectedHandling::Exclude);
}

#[test]
fn add_4_7_the_dimension_of_over_bounds_each_unit() {
    let plan = plan(
        declaration(MissingHandling::Exclude),
        vec![dimension("ORDER_DATE", Some(Grain::Year), 0)],
        BoundQueryParts::default(),
    )
    .expect("a plan");

    assert_eq!(
        time(snapshot_rule(&plan)).unit,
        Some(UnitKey {
            column: ColumnName::new("dimension_1"),
            grain: Grain::Year,
        })
    );
}

#[test]
fn add_4_10_without_a_dimension_of_over_the_range_is_one_unit() {
    let plan = plan(
        declaration(MissingHandling::Exclude),
        vec![dimension("REGION", None, 0)],
        BoundQueryParts::default(),
    )
    .expect("a plan");

    assert_eq!(time(snapshot_rule(&plan)).unit, None);
}

#[test]
fn e4_7_1_each_component_is_recomposed_by_its_own_function() {
    let plan = plan(
        declaration(MissingHandling::Exclude),
        month(),
        BoundQueryParts::default(),
    )
    .expect("a plan");

    let Operation::Aggregate { components, .. } = node(&plan, "Aggregate").operation() else {
        panic!("an Aggregate");
    };
    let functions: Vec<(&str, &AggregateCall)> = components
        .iter()
        .map(|component| (component.name.as_str(), &component.call))
        .collect();
    let sum_of = |column: &str| AggregateCall::Simple {
        function: SimpleAggregate::Sum,
        argument: Box::new(column_reference(column)),
    };
    assert_eq!(
        functions,
        [
            ("metric_1_component_1", &sum_of("metric_1_component_1")),
            ("metric_1_component_2", &sum_of("metric_1_component_2")),
        ]
    );
}

#[test]
fn e4_7_3_the_periods_with_data_are_the_truncated_periods_of_the_rows() {
    let plan = plan(
        declaration(MissingHandling::Exclude),
        month(),
        BoundQueryParts::default(),
    )
    .expect("a plan");

    let time = time(snapshot_rule(&plan));
    let periods = time.expected.inputs()[1];
    assert_eq!(kinds(periods), ["Aggregate", "Scan"]);
    let Operation::Aggregate {
        keys, components, ..
    } = periods.operation()
    else {
        panic!("an Aggregate");
    };
    assert!(components.is_empty());
    assert_eq!(keys[0].expression, time.period);
    assert!(matches!(time.period, Expression::DateTrunc { .. }));
}

#[test]
fn e4_7_5_carried_values_are_read_without_the_date_range_on_over() {
    let carry_forward = MissingHandling::CarryForward {
        max_carry_forward: NonZeroU64::MIN,
        on_stale: StaleHandling::Error,
    };
    let parts = BoundQueryParts {
        date_range: Some(date_range(
            "ORDER_DATE",
            date(2026, 1, 1),
            date(2026, 12, 31),
        )),
        ..BoundQueryParts::default()
    };

    let plan = plan(declaration(carry_forward), month(), parts).expect("a plan");

    let rule = snapshot_rule(&plan);
    assert_eq!(
        kinds(&plan),
        ["Project", "Aggregate", "Snapshot", "Filter", "Scan"]
    );
    let SnapshotMissing::CarryForward { carried, .. } = &time(rule).missing else {
        panic!("carry_forward");
    };
    assert_eq!(kinds(carried), ["Scan"]);
}

#[test]
fn e4_7_6_rows_added_for_missing_zero_come_from_the_expected_pairs() {
    let plan = plan(
        declaration(MissingHandling::Zero),
        month(),
        BoundQueryParts::default(),
    )
    .expect("a plan");

    let SnapshotMissing::Zero { added } = &time(snapshot_rule(&plan)).missing else {
        panic!("zero");
    };
    assert_eq!(kinds(added), ["Project", "Population", "Scan"]);
    let Operation::Project { expressions, .. } = added.operation() else {
        panic!("a Project");
    };
    let value = |column: &str| {
        expressions
            .iter()
            .find(|named| named.name.as_str() == column)
            .map(|named| named.expression.clone())
    };
    assert_eq!(value("REGION"), Some(column_reference("CUSTOMER_ID")));
    assert_eq!(value("ORDER_DATE"), Some(column_reference("period")));
    assert_eq!(value("AMOUNT"), Some(support::tree("NULL")));
}

#[test]
fn e4_7_a_non_time_over_has_no_population() {
    let declared = SemiAdditive {
        over: field_path("STATUS"),
        entity: vec![field_path("REGION")],
        aggregation: SnapshotAggregation::Maximum,
        nulls: NullHandling::Keep,
        snapshot_grain: None,
        population: None,
        missing: None,
        unexpected: None,
    };

    let plan = plan(declared, month(), BoundQueryParts::default()).expect("a plan");

    let rule = snapshot_rule(&plan);
    assert_eq!(rule.over, SnapshotOver::Field(column_reference("STATUS")));
    assert_eq!(node(&plan, "Snapshot").inputs().len(), 1);
}

#[test]
fn e4_5_a_semi_additive_branch_is_stitched_with_the_plain_branch() {
    let branches = Branches::new(vec![branch(BranchKind::Plain, 1, 1), balance_branch(0)])
        .expect("two branches");
    let query = query(
        month(),
        vec![
            metric_reference("BALANCE", 0),
            metric_reference("TOTAL_AMOUNT", 1),
        ],
        BoundQueryParts::default(),
    );

    let plan = build_plan(
        &branches,
        &query,
        &anyone(),
        &model(declaration(MissingHandling::Exclude)),
    )
    .expect("a plan");

    let Operation::Stitch { branches, .. } = node(&plan, "Stitch").operation() else {
        panic!("a Stitch");
    };
    let shapes: Vec<Vec<&str>> = branches.iter().map(kinds).collect();
    assert_eq!(
        shapes,
        [
            vec!["Aggregate", "Scan"],
            vec!["Aggregate", "Snapshot", "Scan"]
        ]
    );
}

fn grained(grain: Grain, entry: Option<&str>) -> BoundDimension {
    let mut dimension = dimension("ORDER_DATE", Some(grain), 1);
    dimension.kind = BoundDimensionKind::Field {
        field: field_path("ORDER_DATE"),
        time_grain: Some(grain),
        time_grain_origin: entry.map(at),
    };
    dimension
}

#[test]
fn e4_03_a_grain_finer_than_the_snapshot_grain_is_located_at_its_time_grains_entry() {
    let entry = "/query/nonportable_extensions/0/data/time_grains/0";
    let dimensions = vec![
        dimension("REGION", None, 0),
        grained(Grain::Week, Some(entry)),
    ];

    let error = plan(
        declaration(MissingHandling::Exclude),
        dimensions,
        BoundQueryParts::default(),
    );

    assert_eq!(error, Err(PlanError::SplitSnapshot(at(entry))));
}

#[test]
fn e4_03_a_plain_over_finer_than_the_snapshot_grain_is_located_at_the_dimension() {
    let error = plan(
        declaration(MissingHandling::Exclude),
        vec![dimension("ORDER_DATE", None, 0)],
        BoundQueryParts::default(),
    );

    assert_eq!(
        error,
        Err(PlanError::SplitSnapshot(at("/query/dimensions/0")))
    );
    let diagnostic = error
        .err()
        .and_then(|error| error.to_diagnostic())
        .expect("a diagnostic");
    assert_eq!(diagnostic.code, "E4-03");
}
