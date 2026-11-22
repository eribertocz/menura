//! The row policies of a query with a semi-additive branch are checked over the whole query:
//! every `ESEC-02` check, populations included, before any `EQRY-03` check (E4-6.9, E4-6.9b,
//! E2-5 row 18).

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
use menura_representations::logical_plan::PlanNode;
use menura_representations::paths::{Branch, BranchKind, Branches, ComponentKey, JoinTree};
use support::population::{active, subscriptions, team_policy};
use support::{
    anyone, build_plan, field_path, metric, metric_reference, metrics, model_with, name, orders,
    policy, query,
};

fn balance() -> Metric {
    let declared = SemiAdditive {
        over: field_path("ORDER_DATE"),
        entity: vec![field_path("REGION")],
        aggregation: SnapshotAggregation::Last(Selection::GlobalPeriod),
        nulls: NullHandling::Keep,
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

/// `ORDERS` with a policy that reads the current date, and `SUBSCRIPTIONS`, which declares
/// `ACTIVE` under a policy that needs the attribute `team`.
fn model() -> InternalModel {
    let mut all = metrics();
    all.push(balance());
    all.sort_by(|left, right| left.name.cmp(&right.name));
    let dated = policy("recent", "order_date <= CURRENT_DATE", None, None);
    model_with(
        vec![
            orders(vec![dated]),
            subscriptions(vec![active(None)], vec![team_policy()]),
        ],
        all,
    )
}

fn plan() -> Result<PlanNode, PlanError> {
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
    let query = query(
        Vec::new(),
        vec![metric_reference("BALANCE", 0)],
        BoundQueryParts::default(),
    );
    build_plan(&branches, &query, &anyone(), &model())
}

#[test]
fn e4_6_9b_a_missing_attribute_of_the_population_comes_before_a_missing_reference_time() {
    assert!(matches!(plan(), Err(PlanError::MissingAttribute(_))));
}
