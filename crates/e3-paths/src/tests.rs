//! Unit tests of stage E3 for the 0.3.0 slice (VS-1), named after the rule they check.

use menura_model::expression::DateValue;
use menura_model::internal_model::FieldPath;
use menura_representations::bound_query::{
    BoundDateRange, BoundMetricKind, BoundPath, BoundQueryParts, MetricPath,
};
use menura_representations::paths::{Branch, Branches, ComponentKey, JoinTree};

use crate::test_support::{
    at, bound_metric, dataset, dimension, false_literal, filter, metric, metric_reference, model,
    name, query,
};
use crate::{InvariantViolation, resolve_paths};

fn key(metric: usize, component: usize) -> ComponentKey {
    ComponentKey { metric, component }
}

fn only_branch(branches: &Branches) -> &Branch {
    match branches.as_slice() {
        [branch] => branch,
        other => panic!("expected one branch, got {}", other.len()),
    }
}

fn filters(filters: Vec<menura_representations::bound_query::BoundFilter>) -> BoundQueryParts {
    BoundQueryParts {
        filters,
        ..BoundQueryParts::default()
    }
}

#[test]
fn e3_1_a_slice_query_needs_no_join_for_its_components_and_filters() {
    let model = model(Vec::new(), vec![metric("REVENUE", &["ORDERS"])]);
    let query = query(
        vec![dimension("ORDERS", "STATUS", 0)],
        vec![metric_reference("REVENUE", 0)],
        filters(vec![filter(&["ORDERS"], 0)]),
    );

    let branches = resolve_paths(&query, &model).expect("a slice query");

    assert_eq!(only_branch(&branches).join_tree, JoinTree::default());
}

#[test]
fn e3_2_the_output_is_the_branches_of_the_bound_query() {
    let model = model(Vec::new(), vec![metric("REVENUE", &["ORDERS"])]);
    let query = query(
        Vec::new(),
        vec![metric_reference("REVENUE", 0)],
        BoundQueryParts::default(),
    );

    let branches = resolve_paths(&query, &model).expect("a slice query");

    let expected = Branches::new(vec![Branch {
        home: name("ORDERS"),
        components: vec![key(0, 0)],
        join_tree: JoinTree::default(),
    }])
    .expect("one branch");
    assert_eq!(branches, expected);
}

#[test]
fn e3_4_1_one_metric_is_one_branch_on_its_home_dataset() {
    let model = model(Vec::new(), vec![metric("REVENUE", &["ORDERS"])]);
    let query = query(
        vec![dimension("ORDERS", "STATUS", 0)],
        vec![metric_reference("REVENUE", 0)],
        BoundQueryParts::default(),
    );

    let branches = resolve_paths(&query, &model).expect("a slice query");

    let branch = only_branch(&branches);
    assert_eq!(branch.home, name("ORDERS"));
    assert_eq!(branch.components, [key(0, 0)]);
}

#[test]
fn e3_4_1_two_metrics_with_several_components_on_one_dataset_are_one_branch_in_key_order() {
    let model = model(
        Vec::new(),
        vec![
            metric("MARGIN", &["ORDERS", "ORDERS", "ORDERS"]),
            metric("REVENUE", &["ORDERS", "ORDERS"]),
        ],
    );
    let query = query(
        vec![dimension("ORDERS", "STATUS", 0)],
        vec![
            metric_reference("REVENUE", 0),
            metric_reference("MARGIN", 1),
        ],
        BoundQueryParts::default(),
    );

    let branches = resolve_paths(&query, &model).expect("a slice query");

    let branch = only_branch(&branches);
    assert_eq!(branch.home, name("ORDERS"));
    assert_eq!(
        branch.components,
        [key(0, 0), key(0, 1), key(1, 0), key(1, 1), key(1, 2)]
    );
}

#[test]
fn e3_4_1_a_dataset_scoped_metric_takes_its_components_from_its_dataset() {
    let orders = dataset("ORDERS", vec![metric("ORDER_COUNT", &["ORDERS", "ORDERS"])]);
    let model = model(vec![orders], Vec::new());
    let path = MetricPath::Dataset {
        dataset: name("ORDERS"),
        metric: name("ORDER_COUNT"),
    };
    let query = query(
        Vec::new(),
        vec![bound_metric(BoundMetricKind::Reference(path), 0)],
        BoundQueryParts::default(),
    );

    let branches = resolve_paths(&query, &model).expect("a slice query");

    assert_eq!(only_branch(&branches).components, [key(0, 0), key(0, 1)]);
}

#[test]
fn e3_4_2_a_constant_filter_reads_no_dataset_and_adds_no_join() {
    let model = model(Vec::new(), vec![metric("REVENUE", &["ORDERS"])]);
    let query = query(
        Vec::new(),
        vec![metric_reference("REVENUE", 0)],
        filters(vec![filter(&[], 0)]),
    );

    let branches = resolve_paths(&query, &model).expect("a slice query");

    let branch = only_branch(&branches);
    assert_eq!(branch.home, name("ORDERS"));
    assert_eq!(branch.join_tree, JoinTree::default());
}

#[test]
fn e3_4_3_a_slice_query_has_exactly_one_branch() {
    let model = model(
        Vec::new(),
        vec![
            metric("MARGIN", &["ORDERS"]),
            metric("REVENUE", &["ORDERS"]),
        ],
    );
    let query = query(
        vec![dimension("ORDERS", "STATUS", 0)],
        vec![
            metric_reference("REVENUE", 0),
            metric_reference("MARGIN", 1),
        ],
        filters(vec![filter(&["ORDERS"], 0)]),
    );

    let branches = resolve_paths(&query, &model).expect("a slice query");

    assert_eq!(branches.as_slice().len(), 1);
}

#[test]
fn e3_5_3_a_dimension_only_query_is_one_branch_with_its_dataset_as_anchor() {
    let model = model(Vec::new(), Vec::new());
    let query = query(
        vec![
            dimension("CUSTOMERS", "COUNTRY", 0),
            dimension("CUSTOMERS", "SEGMENT", 1),
        ],
        Vec::new(),
        filters(vec![filter(&["CUSTOMERS"], 0)]),
    );

    let branches = resolve_paths(&query, &model).expect("a slice query");

    let expected = Branches::dimension_only(name("CUSTOMERS"), JoinTree::default());
    assert_eq!(branches, expected);
}

#[test]
fn e3_5_3_a_dimension_only_query_with_a_constant_filter_keeps_its_anchor() {
    let model = model(Vec::new(), Vec::new());
    let query = query(
        vec![dimension("CUSTOMERS", "COUNTRY", 0)],
        Vec::new(),
        filters(vec![filter(&[], 0)]),
    );

    let branches = resolve_paths(&query, &model).expect("a slice query");

    let branch = only_branch(&branches);
    assert_eq!(branch.home, name("CUSTOMERS"));
    assert!(branch.components.is_empty());
}

#[test]
fn vs_1_a_dimension_on_another_dataset_is_an_invariant_violation() {
    let model = model(Vec::new(), vec![metric("REVENUE", &["ORDERS"])]);
    let query = query(
        vec![dimension("CUSTOMERS", "COUNTRY", 0)],
        vec![metric_reference("REVENUE", 0)],
        BoundQueryParts::default(),
    );

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::SeveralDatasets {
            first: name("CUSTOMERS"),
            second: name("ORDERS"),
            origin: at("/query/metrics/0"),
        })
    );
}

#[test]
fn vs_1_components_on_several_datasets_are_an_invariant_violation() {
    let model = model(Vec::new(), vec![metric("NET", &["ORDERS", "RETURNS"])]);
    let query = query(
        Vec::new(),
        vec![metric_reference("NET", 0)],
        BoundQueryParts::default(),
    );

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::SeveralDatasets {
            first: name("ORDERS"),
            second: name("RETURNS"),
            origin: at("/query/metrics/0"),
        })
    );
}

#[test]
fn vs_1_a_filter_on_another_dataset_is_an_invariant_violation() {
    let model = model(Vec::new(), Vec::new());
    let query = query(
        vec![dimension("ORDERS", "STATUS", 0)],
        Vec::new(),
        filters(vec![filter(&["ORDERS"], 0), filter(&["CUSTOMERS"], 1)]),
    );

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::SeveralDatasets {
            first: name("ORDERS"),
            second: name("CUSTOMERS"),
            origin: at("/query/filters/1"),
        })
    );
}

#[test]
fn vs_1_a_date_range_on_another_dataset_is_an_invariant_violation() {
    let model = model(Vec::new(), vec![metric("REVENUE", &["ORDERS"])]);
    let date = DateValue::new(2026, 1, 1).expect("a date");
    let parts = BoundQueryParts {
        date_range: Some(BoundDateRange {
            field: FieldPath {
                dataset: name("CALENDAR"),
                field: name("DAY"),
            },
            from: date,
            to: date,
            origin: at("/query/nonportable_extensions/0/data/date_range"),
        }),
        ..BoundQueryParts::default()
    };
    let query = query(Vec::new(), vec![metric_reference("REVENUE", 0)], parts);

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::SeveralDatasets {
            first: name("ORDERS"),
            second: name("CALENDAR"),
            origin: at("/query/nonportable_extensions/0/data/date_range"),
        })
    );
}

#[test]
fn vs_1_a_query_without_metrics_and_dimensions_is_an_invariant_violation() {
    let model = model(Vec::new(), Vec::new());
    let query = query(
        Vec::new(),
        Vec::new(),
        filters(vec![filter(&["ORDERS"], 0)]),
    );

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::NoMetricAndNoDimension)
    );
}

#[test]
fn vs_1_a_chosen_path_is_an_invariant_violation() {
    let model = model(Vec::new(), vec![metric("REVENUE", &["ORDERS"])]);
    let origin = at("/query/nonportable_extensions/0/data/paths/0");
    let path = BoundPath::new(
        name("ORDERS"),
        name("CUSTOMERS"),
        vec![name("ORDERS_TO_CUSTOMERS")],
        origin.clone(),
    )
    .expect("one relationship");
    let parts = BoundQueryParts {
        paths: vec![path],
        ..BoundQueryParts::default()
    };
    let query = query(Vec::new(), vec![metric_reference("REVENUE", 0)], parts);

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::ChosenPath { origin })
    );
}

#[test]
fn vs_1_an_inline_metric_is_an_invariant_violation() {
    let model = model(Vec::new(), Vec::new());
    let inline = bound_metric(BoundMetricKind::Inline(false_literal()), 0);
    let query = query(Vec::new(), vec![inline], BoundQueryParts::default());

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::InlineMetric {
            origin: at("/query/metrics/0"),
        })
    );
}

#[test]
fn vs_1_a_metric_missing_from_the_model_is_an_invariant_violation() {
    let model = model(Vec::new(), Vec::new());
    let query = query(
        Vec::new(),
        vec![metric_reference("REVENUE", 0)],
        BoundQueryParts::default(),
    );

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::UnclassifiedMetric {
            origin: at("/query/metrics/0"),
        })
    );
}

#[test]
fn vs_1_an_unclassified_metric_is_an_invariant_violation() {
    let mut unclassified = metric("REVENUE", &["ORDERS"]);
    unclassified.classification = None;
    let model = model(Vec::new(), vec![unclassified]);
    let query = query(
        Vec::new(),
        vec![metric_reference("REVENUE", 0)],
        BoundQueryParts::default(),
    );

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::UnclassifiedMetric {
            origin: at("/query/metrics/0"),
        })
    );
}

#[test]
fn vs_1_a_metric_without_components_is_an_invariant_violation() {
    let model = model(
        Vec::new(),
        vec![metric("RATIO", &[]), metric("REVENUE", &["ORDERS"])],
    );
    let query = query(
        Vec::new(),
        vec![metric_reference("REVENUE", 0), metric_reference("RATIO", 1)],
        BoundQueryParts::default(),
    );

    assert_eq!(
        resolve_paths(&query, &model),
        Err(InvariantViolation::MetricWithoutComponents {
            origin: at("/query/metrics/1"),
        })
    );
}
