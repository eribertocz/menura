//! The explanation of the compiled plan (PLN-1, PLN-7, PLN-7.1 to PLN-7.4, E5-2.2, E5-3.8):
//! the entries every stage records, assembled in step order.

#![cfg(test)]

mod support;

use menura_contracts::types::common::{Decision, Step};
use menura_core::{CompileOptions, CompileResult, Handle, LoadOptions, compile, load};
use support::{
    CATALOG_SCHEMA, MODEL, amount_by_region, analyst, compile_default, context, handle,
    handle_with, plan,
};

/// The files of a conformance model, embedded at build time.
struct ConformanceModel {
    model: &'static [u8],
    catalog_schema: &'static [u8],
    catalog_state: Option<&'static [u8]>,
}

macro_rules! conformance_model {
    ($name:literal) => {
        ConformanceModel {
            model: include_bytes!(concat!(
                "../../../conformance/models/",
                $name,
                "/model.yaml"
            )),
            catalog_schema: include_bytes!(concat!(
                "../../../conformance/models/",
                $name,
                "/catalog-schema.json"
            )),
            catalog_state: None,
        }
    };
    ($name:literal, state) => {
        ConformanceModel {
            catalog_state: Some(include_bytes!(concat!(
                "../../../conformance/models/",
                $name,
                "/catalog-state.json"
            ))),
            ..conformance_model!($name)
        }
    };
}

macro_rules! conformance_context {
    ($model:literal, $context:literal) => {
        include_str!(concat!(
            "../../../conformance/models/",
            $model,
            "/contexts/",
            $context
        ))
    };
}

impl ConformanceModel {
    fn handle(&self) -> Handle {
        load(
            self.model,
            self.catalog_schema,
            self.catalog_state,
            &LoadOptions::default(),
        )
        .unwrap()
    }
}

/// A query document with `time_zone` UTC and the query object `query`.
fn query(query: &str) -> String {
    format!(
        r#"{{"contract": "menura.query", "contract_version": "1.1.0", "time_zone": "UTC",
            "query": {query}}}"#
    )
}

/// The explanation of compiling `query` for `context` over `handle`.
fn explanation(handle: &Handle, query: &str, context: &str) -> Vec<Decision> {
    match compile(
        handle,
        query.as_bytes(),
        context.as_bytes(),
        &CompileOptions::default(),
    ) {
        CompileResult::Plan(plan) => plan.explanation,
        CompileResult::Error(error) => panic!("the query does not compile: {error:?}"),
    }
}

fn codes(entries: &[Decision]) -> Vec<&str> {
    entries
        .iter()
        .map(|entry| entry.code.as_deref().unwrap())
        .collect()
}

fn entry<'entries>(entries: &'entries [Decision], code: &str) -> &'entries Decision {
    entries
        .iter()
        .find(|entry| entry.code.as_deref() == Some(code))
        .unwrap_or_else(|| panic!("no `{code}` entry in {entries:?}"))
}

fn subjects<'entries>(entries: &'entries [Decision], code: &str) -> Vec<&'entries str> {
    entries
        .iter()
        .filter(|entry| entry.code.as_deref() == Some(code))
        .map(|entry| entry.subject.as_deref().unwrap())
        .collect()
}

#[test]
fn pln_7_3_the_load_option_is_always_recorded() {
    let entries = plan(compile_default(&handle(), &amount_by_region(), &analyst())).explanation;
    assert_eq!(codes(&entries), ["DEV-4"]);
    let recorded = &entries[0];
    assert_eq!(recorded.step, Step::E0);
    assert_eq!(
        recorded.subject.as_deref(),
        Some("load_option:grant_authenticated_to_all")
    );
    assert_eq!(
        recorded.decision,
        "datasets without access are inaccessible"
    );
}

#[test]
fn pln_7_3_the_load_option_records_its_value() {
    let options = LoadOptions {
        grant_authenticated_to_all: true,
        ..LoadOptions::default()
    };
    let handle = handle_with(MODEL, CATALOG_SCHEMA, None, &options);
    let entries = plan(compile_default(&handle, &amount_by_region(), &analyst())).explanation;
    assert_eq!(
        entry(&entries, "DEV-4").decision,
        "datasets without access are read as authenticated"
    );
}

#[test]
fn pln_7_3_undeclared_roles_and_attributes_are_recorded_in_code_point_order() {
    let user = context(
        "analyst-1",
        &["zeta", "analyst", "auditor"],
        r#"{"regions": ["NORTH"], "team": "a", "Desk": "b"}"#,
    );
    let entries = plan(compile_default(&handle(), &amount_by_region(), &user)).explanation;
    assert_eq!(
        codes(&entries),
        ["DEV-4", "SEC-3.2", "SEC-3.2", "SEC-3.2a", "SEC-3.2a"]
    );
    assert_eq!(subjects(&entries, "SEC-3.2"), ["role:auditor", "role:zeta"]);
    assert_eq!(
        subjects(&entries, "SEC-3.2a"),
        ["attribute:Desk", "attribute:team"]
    );
    assert!(entries[1..].iter().all(|entry| entry.step == Step::E2));
}

#[test]
fn pln_7_3_a_resolved_relative_range_is_recorded_without_its_dates() {
    let handle = conformance_model!("slice").handle();
    let query = r#"{"contract": "menura.query", "contract_version": "1.1.0",
        "reference_time": "2026-02-20T12:00:00-04:00", "time_zone": "UTC",
        "query": {"dimensions": ["sales.region"],
          "metrics": [{"metric": "sale_count", "alias": "sale_count"}],
          "nonportable_extensions": [{"vendor_name": "COMMON", "data": {"namespace": "menura",
            "version": 1, "date_range": {"dimension": "sales.sale_date",
            "relative": {"unit": "month", "count": 1}}}}]}}"#;
    let entries = explanation(
        &handle,
        query,
        conformance_context!("slice", "auditor.json"),
    );
    let resolved = entry(&entries, "QRY-4.2");
    assert_eq!(resolved.step, Step::E2);
    assert_eq!(resolved.subject.as_deref(), Some("field:SALES.SALE_DATE"));
    assert!(resolved.reason.contains("month"));
    assert!(!resolved.reason.contains("2026"));
}

#[test]
fn pln_7_3_an_accepted_path_entry_is_recorded() {
    let handle = conformance_model!("e3-path-choice").handle();
    let query = query(
        r#"{"dimensions": ["calendar.month"],
          "metrics": [{"metric": "total_amount", "alias": "total_amount"}],
          "nonportable_extensions": [{"vendor_name": "COMMON", "data": {"namespace": "menura",
            "version": 1, "paths": [{"from": "orders", "to": "calendar",
            "relationships": ["order_date"]}]}}]}"#,
    );
    let entries = explanation(
        &handle,
        &query,
        conformance_context!("e3-path-choice", "analyst.json"),
    );
    assert_eq!(entry(&entries, "E3-3.3").step, Step::E3);
    assert_eq!(subjects(&entries, "E3-3.3"), ["path:ORDERS->CALENDAR"]);
}

#[test]
fn pln_7_3_plan_decisions_and_rejected_rewrites_are_recorded_in_step_order() {
    let handle = conformance_model!("e5-inner-joins", state).handle();
    let query = query(
        r#"{"dimensions": ["stores.region"],
          "metrics": [{"metric": "total_amount", "alias": "total_amount"}]}"#,
    );
    let entries = explanation(
        &handle,
        &query,
        conformance_context!("e5-inner-joins", "analyst.json"),
    );
    assert_eq!(
        codes(&entries),
        ["DEV-4", "E4-4.0e", "E4-4.1", "E4-6.10", "E5-3.2"]
    );
    assert_eq!(subjects(&entries, "E4-4.0e"), ["dataset:ORDERS"]);
    assert_eq!(subjects(&entries, "E4-4.1"), ["relationship:ORDERS_STORES"]);
    assert_eq!(subjects(&entries, "E4-6.10"), ["policy:STORES.north_only"]);
    assert!(entry(&entries, "E4-6.10").reason.contains("ORDERS"));
    let rejected = entry(&entries, "E5-3.2");
    assert_eq!(rejected.step, Step::E5);
    assert_eq!(rejected.subject.as_deref(), Some("dataset:ORDERS"));
    assert_eq!(rejected.decision, "rejected");
}

#[test]
fn e5_3_8_an_applied_rewrite_is_recorded_after_the_entry_it_changes() {
    let handle = conformance_model!("e5-inner-joins", state).handle();
    let query = query(
        r#"{"dimensions": ["customers.segment"],
          "metrics": [{"metric": "total_amount", "alias": "total_amount"}]}"#,
    );
    let entries = explanation(
        &handle,
        &query,
        conformance_context!("e5-inner-joins", "analyst.json"),
    );
    assert_eq!(
        codes(&entries),
        ["DEV-4", "E4-4.0e", "E4-4.1", "E4-6.10", "E5-3.2", "E5-3.3"]
    );
    let applied = entry(&entries, "E5-3.3");
    assert_eq!(
        applied.subject.as_deref(),
        Some("relationship:ORDERS_CUSTOMERS")
    );
    assert_eq!(applied.decision, "applied");
}

#[test]
fn e5_2_2_a_summary_is_recorded_with_the_authorized_executor_last() {
    let handle = conformance_model!("e5-summary-selection", state).handle();
    let query = query(
        r#"{"dimensions": ["orders.region"],
          "metrics": [{"metric": "total_amount", "alias": "total_amount"}],
          "filter": "orders.region = 'SOUTH'"}"#,
    );
    let entries = explanation(
        &handle,
        &query,
        conformance_context!("e5-summary-selection", "analyst.json"),
    );
    let used = entries
        .iter()
        .find(|entry| entry.code.as_deref() == Some("E5-3.5") && entry.decision == "applied")
        .unwrap();
    let last = entries.last().unwrap();
    assert_eq!(last.code.as_deref(), Some("E5-4"));
    assert_eq!(last.step, Step::E5);
    assert_eq!(last.subject, used.subject);
    assert_eq!(last.decision, "requires authorized executor");
}

#[test]
fn e5_3_8_a_folded_branch_is_recorded() {
    let handle = conformance_model!("e5-fold-constants").handle();
    let query = query(
        r#"{"dimensions": ["customers.segment"],
          "metrics": [{"metric": "order_count", "alias": "order_count"},
                      {"metric": "total_amount", "alias": "total_amount"}]}"#,
    );
    let entries = explanation(
        &handle,
        &query,
        conformance_context!("e5-fold-constants", "guest.json"),
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.code.as_deref() == Some("E5-3.6") && entry.decision == "applied")
    );
}

#[test]
fn pln_7_1_entries_are_ordered_by_step_without_repetition() {
    let handle = conformance_model!("e5-merge-scans").handle();
    let query = query(
        r#"{"dimensions": ["customers.category"],
          "metrics": [{"metric": "total_paid", "alias": "total_paid"},
                      {"metric": "last_global", "alias": "last_global"},
                      {"metric": "paid_rows", "alias": "paid_rows"}]}"#,
    );
    let entries = explanation(
        &handle,
        &query,
        conformance_context!("e5-merge-scans", "analyst.json"),
    );
    let steps: Vec<Step> = entries.iter().map(|entry| entry.step).collect();
    let mut sorted = steps.clone();
    sorted.sort_by_key(|step| *step as u8);
    assert_eq!(steps, sorted);
    for (index, entry) in entries.iter().enumerate() {
        assert!(
            !entries[..index]
                .iter()
                .any(|earlier| earlier.code == entry.code && earlier.subject == entry.subject),
            "{entry:?} is written twice"
        );
    }
    assert!(codes(&entries).contains(&"E5-3.4"));
}

#[test]
fn pln_7_4_the_explanation_is_deterministic() {
    let handle = conformance_model!("e5-inner-joins", state).handle();
    let query = query(
        r#"{"dimensions": ["stores.region"],
          "metrics": [{"metric": "total_amount", "alias": "total_amount"}]}"#,
    );
    let context = conformance_context!("e5-inner-joins", "analyst.json");
    assert_eq!(
        explanation(&handle, &query, context),
        explanation(&handle, &query, context)
    );
}
