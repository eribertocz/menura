//! The `Population` node: the expected (entity, period) pairs of a population over the
//! visible rows of its dataset (E4-3.12, POP-3, E2-4.9).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_e4_plan::PlanError;
use menura_expressions::typing::big_integer;
use menura_model::internal_model::{PartialPeriod, RoleName, RowPolicy};
use menura_representations::logical_plan::{Membership, Operation};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;
use support::plan::{column, column_names, kind, kinds, predicate};
use support::population::{
    POPULATION_POINTER, active, model_of, periodic, periods, plan, population, rule, subscriptions,
    team_policy, validity, visible_input,
};
use support::propagation::{linked_model, outline, semi_join_relationships, stores};
use support::{anyone, located, name, orders, tree, user};

#[test]
fn e4_3_12_population_outputs_entity_then_period_over_visible_rows_and_periods() {
    let model = model_of(subscriptions(vec![active(None)], Vec::new()));
    let node = plan("ACTIVE", &model, &anyone()).expect("a population plan");

    assert_eq!(kind(&node), "Population");
    assert_eq!(node.origin(), &Origin::Model(located(POPULATION_POINTER)));
    assert_eq!(
        node.schema().columns(),
        [
            column("CUSTOMER_ID", big_integer(), false),
            column("PLAN", LogicalType::Text, true),
            column("PERIOD_START", LogicalType::Date, false),
        ]
    );
    let inputs: Vec<&str> = node.inputs().into_iter().map(kind).collect();
    assert_eq!(inputs, ["Scan", "Scan"]);
    assert!(matches!(
        node.operation(),
        Operation::Population { dataset, population, grain: Grain::Month, period, .. }
            if dataset == &name("SUBSCRIPTIONS")
                && population == &name("ACTIVE")
                && period == &ColumnName::new("PERIOD_START")
    ));
    assert_eq!(node.inputs().get(1).copied(), Some(&periods()));
}

#[test]
fn pop_3_1_validity_reads_its_interval_and_partial_period() {
    for partial_period in [
        PartialPeriod::AnyDay,
        PartialPeriod::FullPeriod,
        PartialPeriod::PeriodEnd,
    ] {
        let model = model_of(subscriptions(
            vec![population("ACTIVE", validity(partial_period), None)],
            Vec::new(),
        ));
        let node = plan("ACTIVE", &model, &anyone()).expect("a population plan");

        assert_eq!(
            rule(&node).membership,
            Membership::Validity {
                valid_from: ColumnName::new("VALID_FROM"),
                valid_to: ColumnName::new("VALID_TO"),
                partial_period,
            }
        );
        assert_eq!(
            column_names(visible_input(&node)),
            ["CUSTOMER_ID", "PLAN", "VALID_FROM", "VALID_TO"]
        );
    }
}

#[test]
fn pop_3_2_periodic_reads_the_period_of_each_row() {
    let model = model_of(subscriptions(
        vec![population("MONTHLY", periodic(), None)],
        Vec::new(),
    ));
    let node = plan("MONTHLY", &model, &anyone()).expect("a population plan");

    assert_eq!(
        rule(&node).membership,
        Membership::Periodic {
            period: ColumnName::new("PERIOD"),
        }
    );
    assert_eq!(
        column_names(visible_input(&node)),
        ["CUSTOMER_ID", "PERIOD", "PLAN"]
    );
}

#[test]
fn pop_3_3_required_when_is_kept_over_the_columns_of_the_rows() {
    for mode in [validity(PartialPeriod::AnyDay), periodic()] {
        let model = model_of(subscriptions(
            vec![population(
                "ACTIVE",
                mode,
                Some("is_billable AND plan <> 'free'"),
            )],
            Vec::new(),
        ));
        let node = plan("ACTIVE", &model, &anyone()).expect("a population plan");

        assert_eq!(
            rule(&node).required_when,
            Some(tree(r#""IS_BILLABLE" AND "PLAN" <> 'free'"#))
        );
        assert!(column_names(visible_input(&node)).contains(&"IS_BILLABLE"));
    }
}

#[test]
fn pop_3_3_without_required_when_every_entity_is_a_candidate() {
    let model = model_of(subscriptions(vec![active(None)], Vec::new()));
    let node = plan("ACTIVE", &model, &anyone()).expect("a population plan");

    assert_eq!(rule(&node).required_when, None);
    assert_eq!(
        rule(&node).entity,
        [ColumnName::new("CUSTOMER_ID"), ColumnName::new("PLAN")]
    );
}

#[test]
fn pop_3_4_every_validity_row_of_an_entity_is_read() {
    let model = model_of(subscriptions(vec![active(None)], Vec::new()));
    let node = plan("ACTIVE", &model, &anyone()).expect("a population plan");

    let input = visible_input(&node);
    assert_eq!(kinds(input), ["Scan"]);
    assert!(matches!(
        input.operation(),
        Operation::Scan {
            row_identity: None,
            ..
        }
    ));
}

#[test]
fn e2_4_9_the_population_read_applies_the_row_policies_of_its_dataset() {
    let model = model_of(subscriptions(vec![active(None)], vec![team_policy()]));
    let node =
        plan("ACTIVE", &model, &user("[]", r#"{"team": "north"}"#)).expect("a population plan");

    let visible = visible_input(&node);
    assert_eq!(kinds(visible), ["Filter", "Scan"]);
    assert!(column_names(visible).contains(&"TEAM"));
    assert_eq!(
        node.schema().columns().len(),
        3,
        "the policy columns never reach the output"
    );
}

#[test]
fn e2_4_9_opposite_users_read_opposite_populations() {
    let mut granted = team_policy();
    granted.condition = Some(tree("plan = 'gold'"));
    granted.roles = Some(vec![RoleName::new("analyst")]);
    let model = model_of(subscriptions(vec![active(None)], vec![granted]));
    let analyst = plan("ACTIVE", &model, &user(r#"["analyst"]"#, "{}")).expect("a population plan");
    let auditor = plan("ACTIVE", &model, &user(r#"["auditor"]"#, "{}")).expect("a population plan");

    let sees = predicate(visible_input(&analyst));
    let denied = predicate(visible_input(&auditor));
    assert_eq!(sees, &tree(r#""PLAN" = 'gold' AND TRUE"#));
    assert_eq!(denied, &tree("FALSE AND TRUE"));
}

#[test]
fn e2_4_9_a_population_policy_needing_an_absent_attribute_fails_with_esec_02() {
    let model = model_of(subscriptions(vec![active(None)], vec![team_policy()]));
    let error = plan("ACTIVE", &model, &anyone()).expect_err("a missing attribute");

    assert!(matches!(error, PlanError::MissingAttribute(_)));
    let diagnostic = error.to_diagnostic().expect("a diagnostic");
    assert_eq!(diagnostic.code, "ESEC-02");
}

#[test]
fn e4_6_5_the_population_read_propagates_the_policies_of_related_datasets() {
    let mut dataset = subscriptions(vec![active(None)], Vec::new());
    dataset.location = located("/semantic_model/0/datasets/1");
    let mut related = stores();
    related.row_policies = vec![RowPolicy {
        name: "region_scope".to_owned(),
        condition: Some(tree("region = USER_ATTRIBUTE('region')")),
        roles: None,
        exempt_roles: None,
        location: located("/semantic_model/0/datasets/2/custom_extensions/0/data"),
    }];
    let model = linked_model(
        vec![orders(Vec::new()), related, dataset],
        &[(
            "SUBSCRIPTIONS_TO_STORES",
            ("SUBSCRIPTIONS", "STORE_ID"),
            ("STORES", "STORE_ID"),
        )],
    );
    let node =
        plan("ACTIVE", &model, &user("[]", r#"{"region": "east"}"#)).expect("a population plan");

    assert_eq!(
        semi_join_relationships(visible_input(&node)),
        ["SUBSCRIPTIONS_TO_STORES"]
    );
    assert!(outline(visible_input(&node)).contains("STORE_ID"));
}

#[test]
fn e4_3_12_an_unknown_population_is_an_internal_error() {
    let model = model_of(subscriptions(vec![active(None)], Vec::new()));
    let error = plan("MISSING", &model, &anyone()).expect_err("an internal error");

    assert!(matches!(error, PlanError::Invariant { .. }));
    assert_eq!(error.to_diagnostic(), None);
}
