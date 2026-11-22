//! E2-5 row 12c, E2-5.4, E2-5.4a, VS-1c and VS-2: several datasets, `paths`, derived metrics,
//! additivity and populations are in the 0.5.0 slice; the first element that needs a context
//! metric is `E_NOT_YET_SUPPORTED`, located at that element, naming context metrics and 0.8.0.

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_e2_bind::{BindCode, BindDiagnostic};
use menura_model::internal_model::{
    ContextMetric, ContextModifier, DeclaredAdditivity, DerivedMetric, FieldPath, ModelParts,
    NullHandling, RemovedDimensions, SemiAdditive, SnapshotAggregation,
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

fn assert_in_the_slice(query: &QueryTree, model: &menura_model::internal_model::InternalModel) {
    let result = bind_query(query, model, ADMIN);
    assert!(result.is_ok(), "{:?}", result.err());
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
fn vs_1b_a_metric_on_another_dataset_than_the_dimensions_is_in_the_slice() {
    assert_in_the_slice(&revenue_by("customers.region"), &model());
}

#[test]
fn vs_1b_a_metric_whose_components_span_two_datasets_is_in_the_slice() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").classification =
            Some(classification(&["CUSTOMERS", "ORDERS"]));
    });
    let query = tree(
        vec![],
        vec![metric(0, "revenue")],
        QueryTreeParts::default(),
    );
    assert_in_the_slice(&query, &model);
}

#[test]
fn vs_1b_dimensions_on_two_datasets_are_in_the_slice() {
    let query = tree(
        vec![
            dimension(0, "orders.region"),
            dimension(1, "customers.customer_id"),
        ],
        vec![],
        QueryTreeParts::default(),
    );
    assert_in_the_slice(&query, &model());
}

#[test]
fn vs_1b_a_filter_on_another_dataset_is_in_the_slice() {
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
    assert_in_the_slice(&query, &model());
}

#[test]
fn vs_1b_a_date_range_on_another_dataset_is_in_the_slice() {
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
    assert_in_the_slice(&query, &model());
}

#[test]
fn vs_1b_paths_are_in_the_slice() {
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
    assert_in_the_slice(&query, &model());
}

#[test]
fn vs_1b_a_derived_metric_is_in_the_slice() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").derived = Some(DerivedMetric {
            inputs: Vec::new(),
            on_division_by_zero: None,
        });
    });
    assert_in_the_slice(&revenue_by("orders.region"), &model);
}

#[test]
fn vs_1b_a_derived_metric_on_two_datasets_is_in_the_slice() {
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
    assert_in_the_slice(&query, &model);
}

#[test]
fn vs_1c_an_additivity_declaration_is_in_the_slice() {
    let model = model_with(|parts| {
        semantic_metric(parts, "REVENUE").declared_additivity =
            Some(DeclaredAdditivity::NonAdditive);
    });
    assert_in_the_slice(&revenue_by("orders.region"), &model);
}

#[test]
fn vs_1c_a_semi_additive_metric_with_a_population_is_in_the_slice() {
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
    assert_in_the_slice(&revenue_by("orders.region"), &model);
}

#[test]
fn vs_1c_a_context_metric_is_0_8_0() {
    let model = model_with(|parts| {
        semantic_metric(parts, "COST").context = Some(ContextMetric {
            base: name("REVENUE"),
            modifier: prior_period(),
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
fn vs_1f_a_share_of_total_metric_is_in_the_slice() {
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
    assert_in_the_slice(&query, &model);
}

/// Makes COST a context metric over REVENUE.
fn cost_as_a_context_metric(parts: &mut ModelParts) {
    semantic_metric(parts, "COST").context = Some(ContextMetric {
        base: name("REVENUE"),
        modifier: prior_period(),
    });
}

fn assert_context_metric(diagnostic: &BindDiagnostic, pointer: &str) {
    assert_not_yet_supported(diagnostic, pointer, "0.8.0");
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
        cost_as_a_context_metric(parts);
    });
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "revenue"), metric(1, "cost")],
        QueryTreeParts::default(),
    );
    assert_context_metric(&bind_diagnostic(&query, &model, ADMIN), "/query/metrics/1");
}

#[test]
fn e2_5_4a_paths_do_not_belong_to_the_whole_query_from_0_5_0() {
    let model = model_with(cost_as_a_context_metric);
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "cost")],
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
    assert_context_metric(&bind_diagnostic(&query, &model, ADMIN), "/query/metrics/0");
}

#[test]
fn e2_5_4a_several_datasets_do_not_belong_to_the_whole_query_from_0_5_0() {
    let model = model_with(cost_as_a_context_metric);
    let query = tree(
        vec![
            dimension(0, "customers.customer_id"),
            dimension(1, "orders.region"),
        ],
        vec![metric(0, "cost")],
        QueryTreeParts::default(),
    );
    assert_context_metric(&bind_diagnostic(&query, &model, ADMIN), "/query/metrics/0");
}

#[test]
fn e2_5_4a_an_element_with_additivity_and_a_context_metric_names_context_metrics() {
    let model = model_with(|parts| {
        cost_as_a_context_metric(parts);
        semantic_metric(parts, "COST").declared_additivity = Some(DeclaredAdditivity::NonAdditive);
    });
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "cost")],
        QueryTreeParts::default(),
    );
    assert_context_metric(&bind_diagnostic(&query, &model, ADMIN), "/query/metrics/0");
}

/// A `prior_period` modifier, which stays outside the slice until its own issue.
fn prior_period() -> ContextModifier {
    ContextModifier::PriorPeriod {
        time_dimension: FieldPath {
            dataset: name("ORDERS"),
            field: name("ORDER_DATE"),
        },
        grain: Grain::Year,
        offset: std::num::NonZeroU64::MIN,
    }
}
