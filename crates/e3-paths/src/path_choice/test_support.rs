//! Relationship graphs and bound queries shared by the unit tests of path choice.

use menura_model::expression::DateValue;
use menura_model::internal_model::{FieldPath, InternalModel, Metric};
use menura_representations::bound_query::{
    AuthorizedSubgraph, BoundDateRange, BoundDimension, BoundPath, BoundQuery, BoundQueryParts,
};
use menura_representations::paths::{
    Branch, BranchKind, Branches, ComponentKey, JoinStep, QueryElement,
};

use super::BranchHome;
use crate::test_support::{at, connected_model, metric, metric_reference, name, query};
use crate::{PathCode, PathError};

/// The step that crosses `relationship` from `from` to `to`.
pub(crate) fn step(relationship: &str, from: &str, to: &str) -> JoinStep {
    JoinStep {
        relationship: name(relationship),
        from: name(from),
        to: name(to),
    }
}

/// The model with the semantic-model `metrics` and one graph edge for each
/// `(relationship, from, to)` of `edges`, both ordered by name as the internal model orders
/// them (E0H-2.3).
pub(crate) fn graph_model(mut metrics: Vec<Metric>, edges: &[(&str, &str, &str)]) -> InternalModel {
    metrics.sort_by(|left, right| left.name.cmp(&right.name));
    let mut edges = edges.to_vec();
    edges.sort_by_key(|(relationship, _, _)| name(relationship));
    connected_model(Vec::new(), metrics, &edges)
}

/// The subgraph that authorizes every dataset and every relationship of `model`.
pub(crate) fn whole_subgraph(model: &InternalModel) -> AuthorizedSubgraph {
    let graph = &model.parts().graph;
    let mut datasets = Vec::new();
    for edge in &graph.edges {
        datasets.push(edge.from.clone());
        datasets.push(edge.to.clone());
    }
    let relationships = graph
        .edges
        .iter()
        .map(|edge| edge.relationship.clone())
        .collect();
    AuthorizedSubgraph::new(datasets, relationships)
}

/// The `paths` entry at `/query/paths/<index>` from `from` to `to` through `relationships`.
pub(crate) fn chosen(from: &str, to: &str, relationships: &[&str], index: usize) -> BoundPath {
    BoundPath::new(
        name(from),
        name(to),
        relationships
            .iter()
            .map(|relationship| name(relationship))
            .collect(),
        at(&format!("/query/paths/{index}")),
    )
    .expect("at least one relationship")
}

/// The query parts with `paths` and the subgraph that authorizes all of `model`.
pub(crate) fn parts_with_paths(model: &InternalModel, paths: Vec<BoundPath>) -> BoundQueryParts {
    BoundQueryParts {
        paths,
        subgraph: whole_subgraph(model),
        ..BoundQueryParts::default()
    }
}

/// `ORDERS` reaches `CALENDAR` through `SHIP_DATE` and through `ORDER_DATE`, and `CALENDAR`
/// reaches `FISCAL_YEARS`; `ORDERS` also reaches `CUSTOMERS`, which reaches `REGIONS`.
pub(crate) fn shop() -> InternalModel {
    graph_model(
        vec![
            metric("REVENUE", &["ORDERS"]),
            metric("REFUNDS", &["RETURNS"]),
            metric("MIXED", &["RETURNS", "ORDERS"]),
        ],
        &[
            ("SHIP_DATE", "ORDERS", "CALENDAR"),
            ("ORDER_DATE", "ORDERS", "CALENDAR"),
            ("CALENDAR_YEAR", "CALENDAR", "FISCAL_YEARS"),
            ("ORDERS_CUSTOMERS", "ORDERS", "CUSTOMERS"),
            ("CUSTOMERS_REGIONS", "CUSTOMERS", "REGIONS"),
            ("RETURNS_ORDERS", "RETURNS", "ORDERS"),
        ],
    )
}

/// The home `dataset` with the components `(metric, component)`.
pub(crate) fn home(dataset: &str, components: &[(usize, usize)]) -> BranchHome {
    BranchHome {
        home: name(dataset),
        kind: BranchKind::Plain,
        components: components
            .iter()
            .map(|&(metric, component)| ComponentKey { metric, component })
            .collect(),
    }
}

/// The code and pointer of the E3 diagnostic `result` holds.
pub(crate) fn error_at(result: Result<Branches, PathError>) -> (PathCode, String) {
    match result {
        Err(PathError::Diagnostic(diagnostic)) => {
            (diagnostic.code, diagnostic.location.pointer().to_owned())
        }
        other => panic!("expected an E3 diagnostic, got {other:?}"),
    }
}

/// The only branch of `branches`.
pub(crate) fn only_branch(branches: &Branches) -> &Branch {
    match branches.as_slice() {
        [branch] => branch,
        other => panic!("expected one branch, got {}", other.len()),
    }
}

/// The steps to the instance `element` is bound to in `branch`; empty for the home dataset.
pub(crate) fn bound_path(branch: &Branch, element: &QueryElement) -> Vec<JoinStep> {
    let tree = &branch.join_tree;
    tree.bound_instance(element)
        .map(|id| tree.path_to(id).into_iter().cloned().collect())
        .unwrap_or_default()
}

/// A `date_range` on the `DAY` field of `dataset`, written at `/query/date_range`.
pub(crate) fn date_range_on(dataset: &str) -> BoundDateRange {
    BoundDateRange {
        field: FieldPath {
            dataset: name(dataset),
            field: name("DAY"),
        },
        from: DateValue::new(2024, 1, 1).expect("a valid date"),
        to: DateValue::new(2024, 12, 31).expect("a valid date"),
        origin: at("/query/date_range"),
    }
}

/// The `shop` model and a query of `REVENUE` by `dimensions` that chooses `paths`.
pub(crate) fn revenue_by(
    dimensions: Vec<BoundDimension>,
    paths: Vec<BoundPath>,
) -> (InternalModel, BoundQuery) {
    let model = shop();
    let parts = parts_with_paths(&model, paths);
    let query = query(dimensions, vec![metric_reference("REVENUE", 0)], parts);
    (model, query)
}
