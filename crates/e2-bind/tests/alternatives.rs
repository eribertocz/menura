//! PLN-5.4 and PLN-5.4a: an `E_NAME_NOT_FOUND` diagnostic carries the authorized names closest
//! to the reference as written, and a hidden object is never one of them.

// `cfg(test)` marks the helpers as test code, where `expect` and `panic!` are allowed
// (clippy.toml).
#[cfg(test)]
mod support;

use menura_e2_bind::BindCode;
use menura_representations::query_tree::{QueryTree, QueryTreeParts};

use support::diagnostic;
use support::query::{
    column, dimension, equals_text, filter, metric, metric_aggregate, path, tree,
};

const ANALYST: &[&str] = &["analyst"];
const ADMIN: &[&str] = &["admin"];

fn dimension_query(text: &str) -> QueryTree {
    tree(vec![dimension(0, text)], vec![], QueryTreeParts::default())
}

fn metric_query(text: &str) -> QueryTree {
    tree(vec![], vec![metric(0, text)], QueryTreeParts::default())
}

fn path_query(from: &str, relationship: &str) -> QueryTree {
    let parts = QueryTreeParts {
        paths: vec![path(
            "/query/nonportable_extensions/0/data/paths/0",
            from,
            "customers",
            &[relationship],
        )],
        ..QueryTreeParts::default()
    };
    tree(vec![], vec![metric(0, "revenue")], parts)
}

/// The alternatives of the first diagnostic of `query` for a user holding `roles`, which must
/// be `E_NAME_NOT_FOUND`.
fn alternatives(query: &QueryTree, roles: &[&str]) -> Vec<String> {
    let found = diagnostic(query, roles);
    assert_eq!(found.code, BindCode::NameNotFound);
    found.alternatives
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn pln_5_4_dimension_typo_lists_fields() {
    assert_eq!(
        alternatives(&dimension_query("orders.amont"), ANALYST),
        names(&["field:ORDERS.AMOUNT"])
    );
}

#[test]
fn pln_5_4_metric_typo_lists_metrics() {
    assert_eq!(
        alternatives(&metric_query("revenu"), ANALYST),
        names(&["metric:REVENUE"])
    );
    assert_eq!(
        alternatives(&metric_query("orders.order_tota"), ANALYST),
        names(&["metric:ORDERS.ORDER_TOTAL"])
    );
}

#[test]
fn pln_5_4_path_dataset_typo_lists_datasets() {
    assert_eq!(
        alternatives(&path_query("order", "orders_to_customers"), ANALYST),
        names(&["dataset:ORDERS"])
    );
}

#[test]
fn pln_5_4_unknown_relationship_has_no_alternatives() {
    assert!(alternatives(&path_query("orders", "orders_to_customer"), ANALYST).is_empty());
}

#[test]
fn pln_5_4_filter_reference_lists_fields_and_aggregate_lists_metrics() {
    let filtered = |expression| {
        let parts = QueryTreeParts {
            filters: vec![filter("/query/filter", expression)],
            ..QueryTreeParts::default()
        };
        tree(vec![dimension(0, "orders.region")], vec![], parts)
    };
    assert_eq!(
        alternatives(&filtered(equals_text(column("orders.regon"))), ANALYST),
        names(&["field:ORDERS.REGION"])
    );
    assert_eq!(
        alternatives(&filtered(equals_text(metric_aggregate("revenu"))), ANALYST),
        names(&["metric:REVENUE"])
    );
}

#[test]
fn pln_5_4_no_candidate_omits_the_member() {
    let found = diagnostic(&dimension_query("orders.nowhere"), ANALYST);
    assert!(found.alternatives.is_empty());
    assert_eq!(found.to_diagnostic().alternatives, None);
}

#[test]
fn pln_5_4_other_diagnostics_carry_none() {
    let found = diagnostic(&dimension_query("revenue"), ANALYST);
    assert_eq!(found.code, BindCode::MetricInDimensions);
    assert_eq!(found.to_diagnostic().alternatives, None);
}

#[test]
fn pln_5_4_written_alternatives_reach_the_diagnostic() {
    let found = diagnostic(&dimension_query("orders.amont"), ANALYST);
    let written = found.to_diagnostic().alternatives.expect("alternatives");
    assert_eq!(written.0, names(&["field:ORDERS.AMOUNT"]));
}

#[test]
fn pln_5_4a_hidden_field_is_never_a_candidate() {
    assert_eq!(
        alternatives(&dimension_query("orders.unit_cos"), ADMIN),
        names(&["field:ORDERS.UNIT_COST"])
    );
    assert!(alternatives(&dimension_query("orders.unit_cos"), ANALYST).is_empty());
}

#[test]
fn pln_5_4a_hidden_and_missing_names_get_the_same_alternatives() {
    assert_eq!(
        alternatives(&metric_query("secret_total"), ANALYST),
        alternatives(&metric_query("secret_totals"), ANALYST)
    );
    assert_eq!(
        alternatives(&metric_query("secret_tota"), ADMIN),
        names(&["metric:SECRET_TOTAL"])
    );
    assert!(alternatives(&metric_query("secret_tota"), ANALYST).is_empty());
}

#[test]
fn pln_5_4a_hidden_dataset_is_never_a_candidate() {
    assert!(alternatives(&path_query("secret", "orders_to_customers"), ANALYST).is_empty());
}
