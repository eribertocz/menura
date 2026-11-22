//! Unit tests of the bound query (E2-2).

use menura_contracts::types::audit_record::Access;
use menura_contracts::types::common::Grain;
use menura_model::expression::DateValue;
use menura_model::internal_model::{AccessMode, FieldPath, RoleName};

use super::{
    AuthorizationDecision, AuthorizationRule, AuthorizedObject, AuthorizedSubgraph, BoundDateRange,
    BoundDimension, BoundDimensionKind, BoundFilter, BoundMetric, BoundMetricKind, BoundPath,
    BoundQuery, BoundQueryParts, MetricPath,
};
use crate::columns::{Columns, OutputColumn};
use crate::names::OutputName;
use crate::test_support::{at, madrid_time, name, true_literal};

fn field(dataset: &str, field: &str) -> FieldPath {
    FieldPath {
        dataset: name(dataset),
        field: name(field),
    }
}

fn bound_columns() -> Columns<BoundDimension, BoundMetric> {
    Columns::dimensions_then_metrics(
        vec![BoundDimension {
            kind: BoundDimensionKind::Field {
                field: field("ORDERS", "ORDERED_AT"),
                time_grain: Some(Grain::Month),
                time_grain_origin: None,
            },
            output_name: OutputName::new("ordered_at"),
            origin: at("/query/dimensions/0"),
        }],
        vec![BoundMetric {
            kind: BoundMetricKind::Reference(MetricPath::SemanticModel(name("REVENUE"))),
            output_name: OutputName::new("revenue"),
            origin: at("/query/metrics/0"),
        }],
    )
}

fn decision(object: AuthorizedObject) -> AuthorizationDecision {
    AuthorizationDecision {
        object,
        access: Access::Granted,
        rule: AuthorizationRule::DatasetAccess(AccessMode::Roles(vec![RoleName::new("ANALYST")])),
    }
}

#[test]
fn e2_2_every_reference_points_to_a_model_element() {
    let parts = BoundQueryParts {
        filters: vec![BoundFilter {
            expression: true_literal(),
            datasets: vec![name("ORDERS")],
            origin: at("/query/filter"),
        }],
        date_range: Some(BoundDateRange {
            field: field("ORDERS", "ORDERED_AT"),
            from: DateValue::new(2026, 1, 1).expect("a date"),
            to: DateValue::new(2026, 3, 31).expect("a date"),
            origin: at("/query/nonportable_extensions/0/data/date_range"),
        }),
        paths: vec![
            BoundPath::new(
                name("ORDERS"),
                name("CUSTOMERS"),
                vec![name("ORDERS_TO_CUSTOMERS")],
                at("/query/nonportable_extensions/0/data/paths/0"),
            )
            .expect("one relationship"),
        ],
        ..BoundQueryParts::default()
    };
    let query = BoundQuery::new(bound_columns(), madrid_time(), parts);

    let dimension = query.columns().dimensions().next().expect("one dimension");
    assert_eq!(
        dimension.kind,
        BoundDimensionKind::Field {
            field: field("ORDERS", "ORDERED_AT"),
            time_grain: Some(Grain::Month),
            time_grain_origin: None,
        }
    );
    assert!(matches!(
        query.columns().get(1),
        Some(OutputColumn::Metric(BoundMetric {
            kind: BoundMetricKind::Reference(MetricPath::SemanticModel(_)),
            ..
        }))
    ));
    let filter = query.filters().first().expect("one filter");
    assert_eq!(filter.datasets, [name("ORDERS")]);
    assert_eq!(
        query.date_range().map(|range| range.to),
        DateValue::new(2026, 3, 31)
    );
    let path = query.paths().first().expect("one path");
    assert_eq!(path.relationships(), [name("ORDERS_TO_CUSTOMERS")]);
}

#[test]
fn e2_2_keeps_every_authorization_decision_in_order() {
    let parts = BoundQueryParts {
        authorization: vec![
            decision(AuthorizedObject::Dataset(name("ORDERS"))),
            decision(AuthorizedObject::Field(field("ORDERS", "ORDERED_AT"))),
            decision(AuthorizedObject::Metric(MetricPath::Dataset {
                dataset: name("ORDERS"),
                metric: name("REVENUE"),
            })),
        ],
        ..BoundQueryParts::default()
    };
    let query = BoundQuery::new(bound_columns(), madrid_time(), parts);

    let objects: Vec<&AuthorizedObject> = query
        .authorization()
        .iter()
        .map(|decision| &decision.object)
        .collect();
    assert_eq!(
        objects,
        [
            &AuthorizedObject::Dataset(name("ORDERS")),
            &AuthorizedObject::Field(field("ORDERS", "ORDERED_AT")),
            &AuthorizedObject::Metric(MetricPath::Dataset {
                dataset: name("ORDERS"),
                metric: name("REVENUE"),
            }),
        ]
    );
}

#[test]
fn e2_2_the_authorized_subgraph_is_ordered_and_answers_membership() {
    let subgraph = AuthorizedSubgraph::new(
        vec![name("ORDERS"), name("CUSTOMERS"), name("ORDERS")],
        vec![name("ORDERS_TO_CUSTOMERS")],
    );
    let parts = BoundQueryParts {
        subgraph,
        ..BoundQueryParts::default()
    };
    let query = BoundQuery::new(bound_columns(), madrid_time(), parts);

    assert_eq!(
        query.subgraph().datasets(),
        [name("CUSTOMERS"), name("ORDERS")]
    );
    assert!(query.subgraph().contains_dataset(&name("ORDERS")));
    assert!(!query.subgraph().contains_dataset(&name("PAYMENTS")));
    assert!(
        query
            .subgraph()
            .contains_relationship(&name("ORDERS_TO_CUSTOMERS"))
    );
    assert!(
        !query
            .subgraph()
            .contains_relationship(&name("ORDERS_TO_REGIONS"))
    );
}

#[test]
fn e2_2_carries_the_time_zone_and_the_reference_time() {
    let query = BoundQuery::new(bound_columns(), madrid_time(), BoundQueryParts::default());

    assert_eq!(query.time(), &madrid_time());
}
