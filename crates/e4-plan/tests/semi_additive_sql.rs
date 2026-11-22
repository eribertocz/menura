//! A semi-additive metric in a query of the SQL form: `DATE_TRUNC('<grain>', over)` counts as
//! that grain (E4-7.8).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::Grain;
use menura_e4_plan::PlanError;
use menura_model::internal_model::{
    DeclaredAdditivity, InternalModel, Metric, MissingHandling, NullHandling, Selection,
    SemiAdditive, SnapshotAggregation, UnexpectedHandling,
};
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::logical_plan::{
    Operation, PlanNode, SnapshotOver, TimeSnapshot, UnitKey,
};
use menura_representations::names::ColumnName;
use menura_representations::paths::{Branch, BranchKind, Branches, ComponentKey, JoinTree};
use support::plan::node;
use support::population::{active, subscriptions};
use support::sql::{SqlColumn, aggregated, expression_dimension, select_item, sql_query};
use support::{anyone, build_plan, field_path, metric, metrics, model_with, name, orders};

fn balance() -> Metric {
    let declared = SemiAdditive {
        over: field_path("ORDER_DATE"),
        entity: vec![field_path("REGION")],
        aggregation: SnapshotAggregation::Last(Selection::GlobalPeriod),
        nulls: NullHandling::AsMissing,
        snapshot_grain: Some(Grain::Month),
        population: Some(name("ACTIVE")),
        missing: Some(MissingHandling::Exclude),
        unexpected: Some(UnexpectedHandling::Exclude),
    };
    Metric {
        declared_additivity: Some(DeclaredAdditivity::SemiAdditive(declared)),
        ..metric("BALANCE", "SUM(orders.amount)", support::decimal_amount())
    }
}

fn model() -> InternalModel {
    let mut all = metrics();
    all.push(balance());
    all.sort_by(|left, right| left.name.cmp(&right.name));
    model_with(
        vec![
            orders(Vec::new()),
            subscriptions(vec![active(None)], Vec::new()),
        ],
        all,
    )
}

fn plan(dimension: &str) -> Result<PlanNode, PlanError> {
    let branch = Branch {
        home: name("ORDERS"),
        kind: BranchKind::SemiAdditive {
            metric: name("BALANCE"),
        },
        components: vec![ComponentKey {
            metric: 0,
            component: 0,
        }],
        join_tree: JoinTree::default(),
    };
    let branches = Branches::new(vec![branch]).expect("one branch");
    let columns: Vec<SqlColumn> = vec![
        expression_dimension(dimension, "period", 8),
        aggregated("BALANCE", "balance", 60),
    ];
    let query = sql_query(columns, BoundQueryParts::default());
    build_plan(&branches, &query, &anyone(), &model())
}

fn time(plan: &PlanNode) -> &TimeSnapshot {
    let Operation::Snapshot { rule, .. } = node(plan, "Snapshot").operation() else {
        panic!("a Snapshot");
    };
    match &rule.over {
        SnapshotOver::Time(time) => time,
        SnapshotOver::Field(_) => panic!("a time over"),
    }
}

#[test]
fn e4_7_8_date_trunc_of_over_bounds_each_unit_at_its_grain() {
    let plan = plan(r#"DATE_TRUNC('year', "ORDERS"."ORDER_DATE")"#).expect("a plan");

    assert_eq!(
        time(&plan).unit,
        Some(UnitKey {
            column: ColumnName::new("dimension_1"),
            grain: Grain::Year,
        })
    );
}

#[test]
fn e4_03_date_trunc_finer_than_the_snapshot_grain_is_located_at_the_select_item() {
    let error = plan(r#"DATE_TRUNC('week', "ORDERS"."ORDER_DATE")"#);

    assert_eq!(error, Err(PlanError::SplitSnapshot(select_item(8))));
    let diagnostic = error
        .err()
        .and_then(|error| error.to_diagnostic())
        .expect("a diagnostic");
    assert_eq!(diagnostic.code, "E4-03");
}
