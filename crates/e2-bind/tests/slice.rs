//! VS-1b, VS-1c and VS-1f: several datasets, `paths`, derived metrics, additivity,
//! populations and context metrics are in the 0.8.0 slice, so every query binds and row 12c
//! never applies.

#[cfg(test)]
mod support;

use menura_contracts::types::common::Grain;
use menura_model::internal_model::{
    ContextMetric, ContextModifier, DeclaredAdditivity, DerivedMetric, FieldPath, ModelParts,
    NullHandling, RemovedDimensions, SemiAdditive, SnapshotAggregation,
};
use menura_representations::query_tree::{QueryTree, QueryTreeParts};

use support::bind_query;
use support::model::{classification, model, model_with, name, semantic_metric};
use support::query::{
    absolute_range, column, dimension, equals_text, filter, metric, path, time_grain, tree,
};

const ANALYST: &[&str] = &["analyst"];
const ADMIN: &[&str] = &["admin"];

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
fn vs_1f_a_prior_period_metric_is_in_the_slice() {
    let model = model_with(cost_as_a_context_metric);
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "cost")],
        QueryTreeParts::default(),
    );
    assert_in_the_slice(&query, &model);
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

#[test]
fn vs_1f_a_period_to_date_metric_is_in_the_slice() {
    let model = model_with(|parts| {
        semantic_metric(parts, "COST").context = Some(ContextMetric {
            base: name("REVENUE"),
            modifier: ContextModifier::PeriodToDate {
                time_dimension: FieldPath {
                    dataset: name("ORDERS"),
                    field: name("ORDER_DATE"),
                },
                grain: Grain::Year,
            },
        });
    });
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "cost")],
        QueryTreeParts::default(),
    );
    assert_in_the_slice(&query, &model);
}

/// Makes COST a `prior_period` context metric over REVENUE.
fn cost_as_a_context_metric(parts: &mut ModelParts) {
    semantic_metric(parts, "COST").context = Some(ContextMetric {
        base: name("REVENUE"),
        modifier: ContextModifier::PriorPeriod {
            time_dimension: FieldPath {
                dataset: name("ORDERS"),
                field: name("ORDER_DATE"),
            },
            grain: Grain::Year,
            offset: std::num::NonZeroU64::MIN,
        },
    });
}

#[test]
fn vs_1f_a_context_metric_after_an_additivity_declaration_is_in_the_slice() {
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
    assert_in_the_slice(&query, &model);
}

#[test]
fn vs_1f_a_context_metric_with_paths_is_in_the_slice() {
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
    assert_in_the_slice(&query, &model);
}

#[test]
fn vs_1f_a_context_metric_over_several_datasets_is_in_the_slice() {
    let model = model_with(cost_as_a_context_metric);
    let query = tree(
        vec![
            dimension(0, "customers.customer_id"),
            dimension(1, "orders.region"),
        ],
        vec![metric(0, "cost")],
        QueryTreeParts::default(),
    );
    assert_in_the_slice(&query, &model);
}

#[test]
fn vs_1f_a_context_metric_with_an_additivity_declaration_is_in_the_slice() {
    let model = model_with(|parts| {
        cost_as_a_context_metric(parts);
        semantic_metric(parts, "COST").declared_additivity = Some(DeclaredAdditivity::NonAdditive);
    });
    let query = tree(
        vec![dimension(0, "orders.region")],
        vec![metric(0, "cost")],
        QueryTreeParts::default(),
    );
    assert_in_the_slice(&query, &model);
}
