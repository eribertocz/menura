//! The row policies of the home dataset in the plan (E4-6, SEC-3.4, SEC-3.5, E2-5 row 18).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::compiled_plan::InputDocument;
use menura_e4_plan::{PlanError, build_plan};
use menura_representations::bound_query::{BoundQuery, BoundQueryParts};
use menura_representations::logical_plan::PlanNode;
use menura_representations::origin::Origin;
use menura_security::AdmittedSecurityContext;
use support::plan::{column_names, kinds, node, nodes, predicate};
use support::{
    POLICY_POINTER, branches, dimension, filter, located, metric_reference, model, policy, query,
    tree, user,
};

fn region_query() -> BoundQuery {
    query(
        vec![dimension("REGION", None, 0)],
        vec![metric_reference("ORDER_COUNT", 0)],
        BoundQueryParts {
            filters: vec![filter("orders.status = 'paid'", 0)],
            ..BoundQueryParts::default()
        },
    )
}

fn plan_for(
    policies: Vec<menura_model::internal_model::RowPolicy>,
    context: &AdmittedSecurityContext,
) -> Result<PlanNode, PlanError> {
    build_plan(&branches(&[1]), &region_query(), context, &model(policies))
}

fn policy_filter(plan: &PlanNode) -> &PlanNode {
    nodes(plan, "Filter")
        .last()
        .copied()
        .expect("a policy filter")
}

#[test]
fn e4_6_1_the_policy_filter_is_directly_over_the_scan_and_below_the_query_filter() {
    let plan = plan_for(
        vec![policy(
            "team_scope",
            "team = USER_ATTRIBUTE('team')",
            None,
            None,
        )],
        &user("[]", r#"{"team": "north"}"#),
    )
    .expect("a plan");
    assert_eq!(
        kinds(&plan),
        ["Project", "Aggregate", "Filter", "Filter", "Scan"]
    );
    let filters = nodes(&plan, "Filter");
    assert_eq!(predicate(filters[0]), &tree(r#""STATUS" = 'paid'"#));
    let policy_filter = filters[1];
    assert_eq!(
        predicate(policy_filter),
        &tree(r#"TRUE AND "TEAM" = USER_ATTRIBUTE('team')"#)
    );
    assert_eq!(
        policy_filter.origin(),
        &Origin::Model(located(POLICY_POINTER))
    );
    assert_eq!(policy_filter.schema(), node(&plan, "Scan").schema());
}

#[test]
fn e4_6_1_a_dataset_without_policies_has_no_policy_filter() {
    let plan = plan_for(Vec::new(), &user("[]", "{}")).expect("a plan");
    assert_eq!(kinds(&plan), ["Project", "Aggregate", "Filter", "Scan"]);
}

#[test]
fn e4_6_2_two_role_scoped_policies_grant_with_or() {
    let plan = plan_for(
        vec![
            policy(
                "region_scope",
                "region = USER_ATTRIBUTE('region')",
                Some(&["analyst"]),
                None,
            ),
            policy(
                "team_scope",
                "team = USER_ATTRIBUTE('team')",
                Some(&["manager"]),
                None,
            ),
        ],
        &user(r#"["analyst"]"#, r#"{"region": "EU"}"#),
    )
    .expect("a plan");
    assert_eq!(
        predicate(policy_filter(&plan)),
        &tree(r#""REGION" = USER_ATTRIBUTE('region') AND TRUE"#)
    );
    assert_eq!(column_names(node(&plan, "Scan")), ["REGION", "STATUS"]);
}

#[test]
fn e4_6_3_exemption_from_every_role_scoped_policy_makes_g_true() {
    let plan = plan_for(
        vec![policy(
            "region_scope",
            "region = USER_ATTRIBUTE('region')",
            Some(&["analyst"]),
            Some(&["auditor"]),
        )],
        &user(r#"["auditor"]"#, "{}"),
    )
    .expect("a plan");
    assert_eq!(predicate(policy_filter(&plan)), &tree("TRUE AND TRUE"));
}

#[test]
fn e4_6_4_no_grant_is_the_constant_false() {
    let plan = plan_for(
        vec![policy(
            "region_scope",
            "region = USER_ATTRIBUTE('region')",
            Some(&["analyst"]),
            None,
        )],
        &user(r#"["manager"]"#, "{}"),
    )
    .expect("a plan");
    assert_eq!(predicate(policy_filter(&plan)), &tree("FALSE AND TRUE"));
}

#[test]
fn e4_6_9a_an_empty_list_attribute_stays_a_list_parameter() {
    for (condition, expected) in [
        (
            "region IN (USER_ATTRIBUTE('regions'))",
            r#"TRUE AND "REGION" IN (USER_ATTRIBUTE('regions'))"#,
        ),
        (
            "region NOT IN (USER_ATTRIBUTE('regions'))",
            r#"TRUE AND "REGION" NOT IN (USER_ATTRIBUTE('regions'))"#,
        ),
    ] {
        let plan = plan_for(
            vec![policy("regions_scope", condition, None, None)],
            &user("[]", r#"{"regions": []}"#),
        )
        .expect("a plan");
        assert_eq!(predicate(policy_filter(&plan)), &tree(expected));
    }
}

#[test]
fn sec_3_5_no_attribute_value_appears_in_the_plan() {
    let plan = plan_for(
        vec![
            policy(
                "region_scope",
                "region IN (USER_ATTRIBUTE('regions'))",
                None,
                None,
            ),
            policy("team_scope", "team = USER_ATTRIBUTE('team')", None, None),
        ],
        &user(
            "[]",
            r#"{"regions": ["SECRET-EU", "SECRET-US"], "team": "SECRET-TEAM"}"#,
        ),
    )
    .expect("a plan");
    let printed = format!("{plan:?}");
    assert!(!printed.contains("SECRET"));
    assert!(printed.contains("UserAttribute(\"regions\")"));
    assert!(printed.contains("UserAttribute(\"team\")"));
}

#[test]
fn e2_5_row_18_sec_3_4_an_applicable_policy_with_an_absent_attribute_is_esec_02() {
    let error = plan_for(
        vec![policy(
            "team_scope",
            "team = USER_ATTRIBUTE('team')",
            None,
            None,
        )],
        &user("[]", "{}"),
    )
    .expect_err("ESEC-02");
    let diagnostic = error.to_diagnostic().expect("a specification diagnostic");
    assert_eq!(diagnostic.code, "ESEC-02");
    let location = diagnostic.location.expect("a location");
    assert_eq!(location.document, InputDocument::SecurityContext);
    assert_eq!(location.pointer, "/attributes");
    assert!(diagnostic.message.contains("`team`"));
    for hidden in ["team_scope", "ORDERS", "main.orders", "="] {
        assert!(!diagnostic.message.contains(hidden), "{hidden}");
    }
}

#[test]
fn e4_6_9_an_exempt_or_non_granting_policy_never_fails() {
    let plan = plan_for(
        vec![
            policy(
                "region_scope",
                "region = USER_ATTRIBUTE('region')",
                Some(&["analyst"]),
                None,
            ),
            policy(
                "team_scope",
                "team = USER_ATTRIBUTE('team')",
                None,
                Some(&["auditor"]),
            ),
        ],
        &user(r#"["auditor"]"#, "{}"),
    );
    assert!(plan.is_ok());
}

#[test]
fn e4_6_10_the_policy_adds_no_output_column() {
    let plan = plan_for(
        vec![policy(
            "team_scope",
            "team = USER_ATTRIBUTE('team')",
            None,
            None,
        )],
        &user("[]", r#"{"team": "north"}"#),
    )
    .expect("a plan");
    assert_eq!(column_names(&plan), ["region", "order_count"]);
}
