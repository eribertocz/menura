//! A row policy that reads the current date or time needs `reference_time` (E4-6.9b, QRY-2).

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#![cfg(test)]

mod support;

use menura_contracts::types::compiled_plan::InputDocument;
use menura_e4_plan::PlanError;
use menura_model::expression::{TimeOfDay, TimestampValue};
use menura_model::internal_model::RowPolicy;
use menura_representations::bound_query::{BoundQuery, BoundQueryParts};
use menura_representations::columns::Columns;
use menura_representations::logical_plan::PlanNode;
use menura_representations::time::{QueryTime, ReferenceTime, TimeZoneName, UtcOffset};
use menura_security::AdmittedSecurityContext;
use support::{branches, build_plan, date, dimension, metric_reference, model, policy, user};

const SINCE_NOW: &str = "ordered_at <= CURRENT_TIMESTAMP";

/// `REGION` by `ORDER_COUNT`, with `reference_time` when `with_reference_time`.
fn region_query(with_reference_time: bool) -> BoundQuery {
    let reference_time = with_reference_time.then(|| ReferenceTime {
        local: TimestampValue {
            date: date(2026, 11, 15),
            time: TimeOfDay::new(12, 0, 0, "").expect("a time of day"),
        },
        offset: UtcOffset::from_minutes(0).expect("an offset"),
    });
    let time = QueryTime {
        time_zone: TimeZoneName::new("Europe/Madrid"),
        reference_date: reference_time.as_ref().map(|_| date(2026, 11, 15)),
        reference_time,
    };
    BoundQuery::new(
        Columns::dimensions_then_metrics(
            vec![dimension("REGION", None, 0)],
            vec![metric_reference("ORDER_COUNT", 0)],
        ),
        time,
        BoundQueryParts::default(),
    )
}

fn plan_for(
    policies: Vec<RowPolicy>,
    context: &AdmittedSecurityContext,
    with_reference_time: bool,
) -> Result<PlanNode, PlanError> {
    build_plan(
        &branches(&[1]),
        &region_query(with_reference_time),
        context,
        &model(policies),
    )
}

#[test]
fn e4_6_9b_an_applicable_policy_reading_now_without_reference_time_is_eqry_03_at_query() {
    let error = plan_for(
        vec![policy("recent_only", SINCE_NOW, None, None)],
        &user("[]", "{}"),
        false,
    )
    .expect_err("EQRY-03");
    let diagnostic = error.to_diagnostic().expect("a specification diagnostic");
    assert_eq!(diagnostic.code, "EQRY-03");
    let location = diagnostic.location.expect("a location");
    assert_eq!(location.document, InputDocument::Query);
    assert_eq!(location.pointer, "/query");
    for hidden in ["recent_only", "ORDERS", "orders", "main.orders", "<="] {
        assert!(!diagnostic.message.contains(hidden), "{hidden}");
    }
}

#[test]
fn e4_6_9b_a_granting_policy_reading_now_is_eqry_03() {
    let error = plan_for(
        vec![policy(
            "analyst_recent",
            "ordered_at <= CURRENT_DATE",
            Some(&["analyst"]),
            None,
        )],
        &user(r#"["analyst"]"#, "{}"),
        false,
    )
    .expect_err("EQRY-03");
    assert_eq!(
        error.to_diagnostic().expect("a diagnostic").code,
        "EQRY-03"
    );
}

#[test]
fn e4_6_9b_an_exempt_or_non_granting_policy_reading_now_compiles() {
    let plan = plan_for(
        vec![
            policy("exempt_recent", SINCE_NOW, None, Some(&["admin"])),
            policy("analyst_recent", SINCE_NOW, Some(&["analyst"]), None),
            policy("admin_scope", "TRUE", Some(&["admin"]), None),
        ],
        &user(r#"["admin"]"#, "{}"),
        false,
    );
    assert!(plan.is_ok(), "{plan:?}");
}

#[test]
fn e4_6_9b_every_esec_02_check_comes_before_eqry_03() {
    let error = plan_for(
        vec![
            policy("a_recent_only", SINCE_NOW, None, None),
            policy("b_team_scope", "team = USER_ATTRIBUTE('team')", None, None),
        ],
        &user("[]", "{}"),
        false,
    )
    .expect_err("ESEC-02");
    assert_eq!(
        error.to_diagnostic().expect("a diagnostic").code,
        "ESEC-02"
    );
}

#[test]
fn e4_6_9b_an_applicable_policy_reading_now_compiles_with_reference_time() {
    let plan = plan_for(
        vec![policy("recent_only", SINCE_NOW, None, None)],
        &user("[]", "{}"),
        true,
    );
    assert!(plan.is_ok(), "{plan:?}");
}
