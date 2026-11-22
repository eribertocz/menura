//! Unit tests of the branches of semi-additive metrics (E3-4.1, E3-4.3, E4-7.9).

use menura_contracts::types::common::Grain;
use menura_model::internal_model::{
    DeclaredAdditivity, FieldPath, Metric, MissingHandling, NullHandling, Selection, SemiAdditive,
    SnapshotAggregation, UnexpectedHandling,
};
use menura_representations::bound_query::BoundQueryParts;
use menura_representations::paths::{BranchKind, ComponentKey};

use super::key;
use crate::resolve_paths;
use crate::test_support::{dimension, metric, metric_reference, model, name, query};

fn field(dataset: &str, field: &str) -> FieldPath {
    FieldPath {
        dataset: name(dataset),
        field: name(field),
    }
}

fn semi_additive(metric_name: &str) -> Metric {
    Metric {
        declared_additivity: Some(DeclaredAdditivity::SemiAdditive(SemiAdditive {
            over: field("DEBT", "PERIOD"),
            entity: vec![field("DEBT", "CUSTOMER_ID")],
            aggregation: SnapshotAggregation::Last(Selection::GlobalPeriod),
            nulls: NullHandling::Keep,
            snapshot_grain: Some(Grain::Month),
            population: Some(name("ACTIVE_CUSTOMERS")),
            missing: Some(MissingHandling::Exclude),
            unexpected: Some(UnexpectedHandling::Include),
        })),
        ..metric(metric_name, &["DEBT"])
    }
}

fn non_additive(metric_name: &str) -> Metric {
    Metric {
        declared_additivity: Some(DeclaredAdditivity::NonAdditive),
        ..metric(metric_name, &["DEBT"])
    }
}

fn branch_layout(metrics: Vec<Metric>, requested: &[&str]) -> Vec<(BranchKind, Vec<ComponentKey>)> {
    let model = model(Vec::new(), metrics);
    let query = query(
        vec![dimension("DEBT", "PERIOD", 0)],
        requested
            .iter()
            .enumerate()
            .map(|(index, metric_name)| metric_reference(metric_name, index))
            .collect(),
        BoundQueryParts::default(),
    );
    let branches = resolve_paths(&query, &model).expect("reachable metrics");
    branches
        .as_slice()
        .iter()
        .map(|branch| (branch.kind.clone(), branch.components.clone()))
        .collect()
}

fn semi_additive_kind(metric_name: &str) -> BranchKind {
    BranchKind::SemiAdditive {
        metric: name(metric_name),
    }
}

#[test]
fn e3_4_1_a_semi_additive_metric_has_its_own_branch_on_its_home_dataset() {
    let layout = branch_layout(
        vec![semi_additive("BALANCE"), metric("PAYMENTS", &["DEBT"])],
        &["BALANCE", "PAYMENTS"],
    );

    assert_eq!(
        layout,
        [
            (BranchKind::Plain, vec![key(1, 0)]),
            (semi_additive_kind("BALANCE"), vec![key(0, 0)]),
        ]
    );
}

#[test]
fn e3_4_3_semi_additive_branches_follow_the_plain_branch_by_metric_name() {
    let layout = branch_layout(
        vec![
            semi_additive("BALANCE"),
            semi_additive("OVERDUE"),
            metric("PAYMENTS", &["DEBT"]),
        ],
        &["OVERDUE", "PAYMENTS", "BALANCE"],
    );

    let kinds: Vec<BranchKind> = layout.into_iter().map(|(kind, _)| kind).collect();
    assert_eq!(
        kinds,
        [
            BranchKind::Plain,
            semi_additive_kind("BALANCE"),
            semi_additive_kind("OVERDUE"),
        ]
    );
}

#[test]
fn e4_7_9_a_non_additive_metric_shares_the_plain_branch() {
    let layout = branch_layout(
        vec![non_additive("AVERAGE_DEBT"), metric("PAYMENTS", &["DEBT"])],
        &["AVERAGE_DEBT", "PAYMENTS"],
    );

    assert_eq!(layout, [(BranchKind::Plain, vec![key(0, 0), key(1, 0)])]);
}
