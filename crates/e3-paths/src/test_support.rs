//! Models and bound queries shared by the unit tests of this crate.

use menura_contracts::Sha256Hash;
use menura_model::expression::{AggregateCall, Expression, Literal};
use menura_model::internal_model::{
    Additivity, Component, Dataset, DecomposabilityClass, Descriptions, FieldPath, GraphEdge,
    InternalModel, Metric, MetricClassification, ModelParts, NormalizedName, RelationshipGraph,
    SourceLocation,
};
use menura_representations::bound_query::{
    AuthorizedSubgraph, BoundDimension, BoundDimensionKind, BoundFilter, BoundMetric,
    BoundMetricKind, BoundQuery, BoundQueryParts, MetricPath,
};
use menura_representations::columns::Columns;
use menura_representations::names::OutputName;
use menura_representations::origin::QueryLocation;
use menura_representations::time::{QueryTime, TimeZoneName};

/// A normalized name.
pub(crate) fn name(text: &str) -> NormalizedName {
    NormalizedName::new(text)
}

/// A location in the query document, without a text position.
pub(crate) fn at(pointer: &str) -> QueryLocation {
    QueryLocation::new(pointer, None)
}

fn location(pointer: &str) -> SourceLocation {
    SourceLocation {
        pointer: pointer.to_owned(),
        line: 1,
        column: 1,
    }
}

fn component(home_dataset: &str) -> Component {
    Component {
        call: AggregateCall::CountRows,
        filter: None,
        home_dataset: name(home_dataset),
        class: DecomposabilityClass::Distributive,
    }
}

/// A metric named `metric_name` with one `COUNT(*)` component on each of `home_datasets`, in
/// order. Without home datasets it has no components, like a derived metric.
pub(crate) fn metric(metric_name: &str, home_datasets: &[&str]) -> Metric {
    let mut homes: Vec<NormalizedName> = home_datasets.iter().map(|home| name(home)).collect();
    let components = homes.iter().map(|home| component(home.as_str())).collect();
    homes.sort();
    homes.dedup();
    Metric {
        name: name(metric_name),
        expression: None,
        typed: None,
        classification: Some(MetricClassification {
            components,
            inputs: Vec::new(),
            home_datasets: homes,
            additivity: Additivity::Additive,
            recomposable: true,
        }),
        declared_additivity: None,
        derived: None,
        context: None,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location("/semantic_model/0/metrics/0"),
    }
}

/// A dataset named `dataset_name` without fields, declaring the dataset-scoped `metrics`.
pub(crate) fn dataset(dataset_name: &str, metrics: Vec<Metric>) -> Dataset {
    Dataset {
        name: name(dataset_name),
        source: format!("main.{dataset_name}"),
        fields: Vec::new(),
        metrics,
        primary_key: Vec::new(),
        unique_keys: Vec::new(),
        access: None,
        row_policies: Vec::new(),
        populations: Vec::new(),
        summary: false,
        descriptions: Descriptions::default(),
        query_errors: Vec::new(),
        location: location("/semantic_model/0/datasets/0"),
    }
}

/// The model with `datasets` and the semantic-model `metrics`, without relationships.
pub(crate) fn model(datasets: Vec<Dataset>, metrics: Vec<Metric>) -> InternalModel {
    connected_model(datasets, metrics, &[])
}

/// The model with `datasets`, the semantic-model `metrics` and one graph edge for each
/// `(relationship, from, to)` of `edges`.
pub(crate) fn connected_model(
    datasets: Vec<Dataset>,
    metrics: Vec<Metric>,
    edges: &[(&str, &str, &str)],
) -> InternalModel {
    let edges = edges
        .iter()
        .map(|(relationship, from, to)| GraphEdge {
            relationship: name(relationship),
            from: name(from),
            to: name(to),
            one_to_one: false,
        })
        .collect();
    let graph = RelationshipGraph {
        edges,
        ..RelationshipGraph::default()
    };
    let parts = ModelParts {
        name: name("SALES"),
        descriptions: Descriptions::default(),
        datasets,
        relationships: Vec::new(),
        graph,
        metrics,
        security_context: None,
        summaries: Vec::new(),
        checks: Vec::new(),
        unusable: Vec::new(),
        warnings: Vec::new(),
    };
    let hash = Sha256Hash::try_from("0".repeat(64)).expect("a valid hash");
    InternalModel::new(parts, hash)
}

/// The dimension `dataset.field`, without a time grain, written at `/query/dimensions/<index>`.
pub(crate) fn dimension(dataset: &str, field: &str, index: usize) -> BoundDimension {
    BoundDimension {
        kind: BoundDimensionKind::Field {
            field: FieldPath {
                dataset: name(dataset),
                field: name(field),
            },
            time_grain: None,
            time_grain_origin: None,
        },
        output_name: OutputName::new(field),
        origin: at(&format!("/query/dimensions/{index}")),
    }
}

/// The metric of the semantic model named `metric_name`, written at `/query/metrics/<index>`.
pub(crate) fn metric_reference(metric_name: &str, index: usize) -> BoundMetric {
    bound_metric(
        BoundMetricKind::Reference(MetricPath::SemanticModel(name(metric_name))),
        index,
    )
}

/// The metric of kind `kind`, written at `/query/metrics/<index>`.
pub(crate) fn bound_metric(kind: BoundMetricKind, index: usize) -> BoundMetric {
    BoundMetric {
        kind,
        output_name: OutputName::new(&format!("metric_{index}")),
        origin: at(&format!("/query/metrics/{index}")),
    }
}

/// A filter that reads `datasets`, written at `/query/filters/<index>`.
pub(crate) fn filter(datasets: &[&str], index: usize) -> BoundFilter {
    BoundFilter {
        expression: false_literal(),
        datasets: datasets.iter().map(|dataset| name(dataset)).collect(),
        origin: at(&format!("/query/filters/{index}")),
    }
}

/// The literal `FALSE`.
pub(crate) fn false_literal() -> Expression {
    Expression::Literal(Literal::Boolean(false))
}

/// The wide-table query with `dimensions`, `metrics` and `parts`.
pub(crate) fn query(
    dimensions: Vec<BoundDimension>,
    metrics: Vec<BoundMetric>,
    parts: BoundQueryParts,
) -> BoundQuery {
    let time = QueryTime {
        time_zone: TimeZoneName::new("UTC"),
        reference_time: None,
        reference_date: None,
    };
    BoundQuery::new(
        Columns::dimensions_then_metrics(dimensions, metrics),
        time,
        parts,
    )
}

/// The authorized subgraph with the relationships `relationships` and no dataset: E3 reads
/// only its relationships.
pub(crate) fn subgraph(relationships: &[&str]) -> AuthorizedSubgraph {
    AuthorizedSubgraph::new(
        Vec::new(),
        relationships
            .iter()
            .map(|relationship| name(relationship))
            .collect(),
    )
}
