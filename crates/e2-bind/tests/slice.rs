//! E2-5 row 12c, E2-5.4, VS-1 and VS-2: the first element outside the 0.3.0 slice is
//! `E_NOT_YET_SUPPORTED`, located at that element, naming the capability whose version comes
//! first.

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::{BindCode, BindDiagnostic};
use menura_model::internal_model::{
    ContextMetric, ContextModifier, DeclaredAdditivity, DerivedMetric, FieldPath, NullHandling,
    RemovedDimensions, SemiAdditive, SnapshotAggregation,
};
use menura_representations::query_tree::{QueryTree, QueryTreeParts};

use support::model::{classification, model, model_with, name, semantic_metric};
use support::query::{
    absolute_range, column, dimension, equals_text, filter, metric, path, time_grain, tree,
};
use support::{bind_diagnostic, bind_query};

const ANALYST: &[&str] = &["analyst"];
const ADMIN: &[&str] = &["admin"];

fn assert_not_yet_supported(diagnostic: &BindDiagnostic, pointer: &str, version: &str) {
    assert_eq!(diagnostic.code, BindCode::NotYetSupported);
    assert_eq!(diagnostic.location.pointer(), pointer);
    assert!(
        diagnostic
            .message
            .ends_with(&format!("not supported before {version}")),
        "{}",
        diagnostic.message
    );
}

fn revenue_by(dimension_text: &str) -> QueryTree {
    tree(
        vec![dimension(0, dimension_text)],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    )
}

fn field(dataset: &str, field: &str) -> FieldPath {
    FieldPath {
        dataset: name(dataset),
        field: name(field),
    }
}

#[test]
fn vs_1_a_dimension_only_query_on_one_dataset_is_accepted() {
    let query = tree(
        vec![
            dimension(0, "orders.region"),
            dimension(1, "orders.ordered_at"),
        ],
        vec![],
        QueryTreeParts::default(),
    );
    assert!(bind_query(&query, &model(), ANALYST).is_ok());
}

#[test]
fn vs_1_base_metrics_dimensions_filters_and_time_settings_on_one_dataset_are_accepted() {
    let query = tree(
        vec![dimension(0, "orders.ordered_at")],
        vec![metric(0, "revenue"), metric(1, "orders.order_total")],
        QueryTreeParts {
            filters: vec![filter(
                "/query/filter",
                equals_text(column("orders.region")),
            )],
            time_grains: vec![time_grain(
                "/query/nonportable_extensions/0/data/time_grains/0",
                "orders.ordered_at",
                Grain::Week,
            )],
            date_range: Some(absolute_range(
                "/query/nonportable_extensions/0/data/date_range",
                "orders.ordered_at",
                "2026-01-01",
                "2026-12-31",
            )),
            ..QueryTreeParts::default()
        },
    );
    assert!(bind_query(&query, &model(), ANALYST).is_ok());
}

#[test]
fn vs_1_a_metric_on_another_dataset_than_the_dimensions_is_0_4_0_at_the_metric() {
    let diagnostic = bind_diagnostic(&revenue_by("customers.region"), &model(), ANALYST);
    assert_not_yet_supported(&diagnostic, "/query/metrics/0", "0.4.0");
}

#[test]
fn vs_1_a_metric_whose_components_span_two_datasets_is_0_4_0_at_the_metric() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").classification =
            Some(classification(&["CUSTOMERS", "ORDERS"]));
    });
    let query = tree(
        vec![],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    assert_not_yet_supported(
        &bind_diagnostic(&query, &model, ANALYST),
        "/query/metrics/0",
        "0.4.0",
    );
}

#[test]
fn vs_1_dimensions_on_two_datasets_are_0_4_0_at_the_second() {
    let query = tree(
        vec![
            dimension(0, "orders.region"),
            dimension(1, "customers.customer_id"),
        ],
        vec![],
        QueryTreeParts::default(),
    );
    assert_not_yet_supported(
        &bind_diagnostic(&query, &model(), ANALYST),
        "/query/dimensions/1",
        "0.4.0",
    );
}

#[test]
fn vs_1_a_filter_on_another_dataset_is_0_4_0_at_the_filter() {
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            filters: vec![filter(
                "/query/filter",
                equals_text(column("customers.region")),
            )],
            ..QueryTreeParts::default()
        },
    );
    assert_not_yet_supported(
        &bind_diagnostic(&query, &model(), ANALYST),
        "/query/filter",
        "0.4.0",
    );
}

#[test]
fn vs_1_a_date_range_on_another_dataset_is_0_4_0_at_the_date_range() {
    let pointer = "/query/nonportable_extensions/0/data/date_range";
    let query = tree(
        vec![dimension(0, "customers.region")],
        vec![],
        QueryTreeParts {
            date_range: Some(absolute_range(
                pointer,
                "orders.ordered_at",
                "2026-01-01",
                "2026-01-31",
            )),
            ..QueryTreeParts::default()
        },
    );
    assert_not_yet_supported(
        &bind_diagnostic(&query, &model(), ANALYST),
        pointer,
        "0.4.0",
    );
}

#[test]
fn vs_1_paths_are_0_4_0_at_the_path() {
    let pointer = "/query/nonportable_extensions/0/data/paths/0";
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![],
        QueryTreeParts {
            paths: vec![path(
                pointer,
                "orders",
                "customers",
                &["orders_to_customers"],
            )],
            ..QueryTreeParts::default()
        },
    );
    assert_not_yet_supported(&bind_diagnostic(&query, &model(), ADMIN), pointer, "0.4.0");
}

#[test]
fn vs_1_a_derived_metric_is_0_4_0() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").derived = Some(DerivedMetric {
            inputs: Vec::new(),
            on_division_by_zero: None,
        });
    });
    let diagnostic = bind_diagnostic(&revenue_by("orders.region"), &model, ANALYST);
    assert_not_yet_supported(&diagnostic, "/query/metrics/0", "0.4.0");
    assert!(
        diagnostic.message.contains("derived"),
        "{}",
        diagnostic.message
    );
}

#[test]
fn e2_5_4a_a_derived_metric_on_two_datasets_names_several_datasets_and_0_4_0() {
    let model = model_with(|parts| {
        let revenue = semantic_metric(parts, "REVENUE");
        revenue.derived = Some(DerivedMetric {
            inputs: Vec::new(),
            on_division_by_zero: None,
        });
        revenue.classification = Some(classification(&["CUSTOMERS", "ORDERS"]));
    });
    let query = tree(
        vec![],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    let diagnostic = bind_diagnostic(&query, &model, ANALYST);
    assert_not_yet_supported(&diagnostic, "/query/metrics/0", "0.4.0");
    assert!(
        diagnostic.message.contains("more than one dataset"),
        "{}",
        diagnostic.message
    );
}

#[test]
fn vs_1_an_additivity_declaration_is_0_5_0() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").declared_additivity =
            Some(DeclaredAdditivity::NonAdditive);
    });
    let diagnostic = bind_diagnostic(&revenue_by("orders.region"), &model, ANALYST);
    assert_not_yet_supported(&diagnostic, "/query/metrics/0", "0.5.0");
    assert!(
        diagnostic.message.contains("additivity"),
        "{}",
        diagnostic.message
    );
}

#[test]
fn vs_1_a_population_is_0_5_0() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").declared_additivity =
            Some(DeclaredAdditivity::SemiAdditive(SemiAdditive {
                over: field("ORDERS", "ORDERED_AT"),
                entity: vec![field("ORDERS", "CUSTOMER_ID")],
                aggregation: SnapshotAggregation::Minimum,
                nulls: NullHandling::Keep,
                snapshot_grain: None,
                population: Some(name("ACTIVE_CUSTOMERS")),
                missing: None,
                unexpected: None,
            }));
    });
    let diagnostic = bind_diagnostic(&revenue_by("orders.region"), &model, ANALYST);
    assert_not_yet_supported(&diagnostic, "/query/metrics/0", "0.5.0");
}

#[test]
fn vs_1_a_context_metric_is_0_8_0() {
    let model = model_with(|parts| {
        semantic_metric(parts, "COST").context = Some(ContextMetric {
            base: name("REVENUE"),
            modifier: ContextModifier::ShareOfTotal(RemovedDimensions::All),
        });
    });
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "cost")],
        QueryTreeParts::default(),
    );
    let diagnostic = bind_diagnostic(&query, &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "/query/metrics/0", "0.8.0");
    assert!(
        diagnostic.message.contains("context"),
        "{}",
        diagnostic.message
    );
}

#[test]
fn e2_5_4_the_first_element_in_query_order_outside_the_slice_is_reported() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").declared_additivity =
            Some(DeclaredAdditivity::NonAdditive);
        semantic_metric(parts, "COST").derived = Some(DerivedMetric {
            inputs: Vec::new(),
            on_division_by_zero: None,
        });
    });
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue"), metric(1, "cost")],
        QueryTreeParts::default(),
    );
    assert_not_yet_supported(
        &bind_diagnostic(&query, &model, ADMIN),
        "/query/metrics/0",
        "0.5.0",
    );
}

#[test]
fn e2_5_4a_a_semi_additive_metric_with_a_population_names_additivity() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").declared_additivity =
            Some(DeclaredAdditivity::SemiAdditive(SemiAdditive {
                over: field("ORDERS", "ORDERED_AT"),
                entity: vec![field("ORDERS", "CUSTOMER_ID")],
                aggregation: SnapshotAggregation::Minimum,
                nulls: NullHandling::Keep,
                snapshot_grain: None,
                population: Some(name("ACTIVE_CUSTOMERS")),
                missing: None,
                unexpected: None,
            }));
    });
    let diagnostic = bind_diagnostic(&revenue_by("orders.region"), &model, ANALYST);
    assert_not_yet_supported(&diagnostic, "/query/metrics/0", "0.5.0");
    assert!(
        diagnostic.message.contains("additivity"),
        "{}",
        diagnostic.message
    );
}

#[test]
fn e2_5_4_a_derived_metric_before_paths_is_reported_at_the_metric() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").derived = Some(DerivedMetric {
            inputs: Vec::new(),
            on_division_by_zero: None,
        });
    });
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue")],
        QueryTreeParts {
            paths: vec![path(
                "/query/nonportable_extensions/0/data/paths/0",
                "orders",
                "customers",
                &["orders_to_customers"],
            )],
            ..QueryTreeParts::default()
        },
    );
    let diagnostic = bind_diagnostic(&query, &model, ADMIN);
    assert_not_yet_supported(&diagnostic, "/query/metrics/0", "0.4.0");
    assert!(
        diagnostic.message.contains("derived"),
        "{}",
        diagnostic.message
    );
}
