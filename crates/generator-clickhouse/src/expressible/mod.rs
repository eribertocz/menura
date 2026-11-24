//! The check of E6-7.12: a plan that reads, computes or returns a value `ClickHouse` cannot
//! express stops with `E6-01` before any SQL is written.
//!
//! The error is located at the first query element, in the order E2 checks them (E2-5.3),
//! whose evaluation needs such a value: the dimensions, the metrics, the `filters` items, then
//! `date_range`. When no query element needs one, the value is in a row policy or another
//! model expression, and the error is located at the dataset that holds it.

mod needs;
mod walk;

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::internal_model::{Dataset, InternalModel};
use menura_representations::bound_query::{
    BoundDimensionKind, BoundMetricKind, BoundQuery, MetricPath,
};
use menura_representations::columns::OutputColumn;
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::origin::{Origin, QueryLocation};

use crate::error::GenerateError;
use crate::types::is_inexpressible_value;
use needs::{Needs, Scope, outside_date32};

/// `E6-01` when `plan`, the plan of `query` over `model` and `catalog`, needs a value
/// `ClickHouse` cannot express, located as E6-7.12 states.
///
/// # Errors
///
/// [`GenerateError::InexpressibleType`] when it does.
pub(crate) fn check_query(
    plan: &PlanNode,
    query: &BoundQuery,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<(), GenerateError> {
    let needs = Needs {
        model,
        catalog,
        reference_date: query.time().reference_date,
    };
    if let Some((location, subject)) = first_query_element(&needs, query) {
        return Err(GenerateError::InexpressibleType {
            origin: Origin::Query(location),
            subject,
        });
    }
    check_plan(plan, &needs)
}

/// `E6-01` when the integrity-check plan `plan` needs such a value, located at the dataset
/// that holds it (E6-7.11, E6-7.12).
///
/// # Errors
///
/// [`GenerateError::InexpressibleType`] when it does.
pub(crate) fn check_model_plan(
    plan: &PlanNode,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<(), GenerateError> {
    let needs = Needs {
        model,
        catalog,
        reference_date: None,
    };
    check_plan(plan, &needs)
}

/// The location and subject of the first query element that needs such a value.
fn first_query_element(
    needs: &Needs<'_>,
    query: &BoundQuery,
) -> Option<(QueryLocation, Option<String>)> {
    for column in query.columns().iter() {
        let found = match column {
            OutputColumn::Dimension(dimension) => dimension_needs(needs, &dimension.kind)
                .then(|| (dimension.origin.clone(), dimension_subject(&dimension.kind))),
            OutputColumn::Metric(metric) => metric_needs(needs, &metric.kind)
                .then(|| (metric.origin.clone(), metric_subject(&metric.kind))),
        };
        if found.is_some() {
            return found;
        }
    }
    for filter in query.filters() {
        let scope = Scope::Fields(
            filter
                .datasets
                .iter()
                .filter_map(|name| needs.model.dataset(name))
                .collect(),
        );
        if needs.expression(&filter.expression, &scope, true, 0) {
            return Some((filter.origin.clone(), None));
        }
    }
    let range = query.date_range()?;
    let field = needs
        .model
        .dataset(&range.field.dataset)
        .and_then(|dataset| {
            Needs::field_named(dataset, &range.field.field).map(|field| (dataset, field))
        });
    let field_needs = field.is_some_and(|(dataset, field)| needs.field(dataset, field, 0));
    (field_needs || outside_date32(range.from) || outside_date32(range.to)).then(|| {
        (
            range.origin.clone(),
            Some(format!(
                "field:{}.{}",
                range.field.dataset.as_str(),
                range.field.field.as_str()
            )),
        )
    })
}

/// Whether a dimension needs such a value: its field, or its expression.
fn dimension_needs(needs: &Needs<'_>, kind: &BoundDimensionKind) -> bool {
    match kind {
        BoundDimensionKind::Field { field, .. } => {
            needs.model.dataset(&field.dataset).is_some_and(|dataset| {
                Needs::field_named(dataset, &field.field)
                    .is_some_and(|found| needs.field(dataset, found, 0))
            })
        }
        BoundDimensionKind::Expression {
            dataset,
            expression,
        } => {
            let scope = Scope::Fields(needs.model.dataset(dataset).into_iter().collect());
            needs.expression(expression, &scope, true, 0)
        }
    }
}

/// Whether a metric needs such a value: the model metric, or its inline expression.
fn metric_needs(needs: &Needs<'_>, kind: &BoundMetricKind) -> bool {
    match kind {
        BoundMetricKind::Reference(path) => {
            let (dataset, name) = match path {
                MetricPath::SemanticModel(name) => (None, name),
                MetricPath::Dataset { dataset, metric } => (Some(dataset), metric),
            };
            needs
                .metric_named(dataset, name)
                .is_some_and(|metric| needs.metric(metric, 0))
        }
        BoundMetricKind::Inline(expression) => {
            let scope = Scope::Fields(needs.model.parts().datasets.iter().collect());
            needs.expression(expression, &scope, true, 0)
        }
    }
}

/// `field:<dataset>.<field>` for a field dimension.
fn dimension_subject(kind: &BoundDimensionKind) -> Option<String> {
    match kind {
        BoundDimensionKind::Field { field, .. } => Some(format!(
            "field:{}.{}",
            field.dataset.as_str(),
            field.field.as_str()
        )),
        BoundDimensionKind::Expression { .. } => None,
    }
}

/// `metric:<name>` for a model metric.
fn metric_subject(kind: &BoundMetricKind) -> Option<String> {
    match kind {
        BoundMetricKind::Reference(MetricPath::SemanticModel(name)) => {
            Some(format!("metric:{}", name.as_str()))
        }
        BoundMetricKind::Reference(MetricPath::Dataset { dataset, metric }) => {
            Some(format!("metric:{}.{}", dataset.as_str(), metric.as_str()))
        }
        BoundMetricKind::Inline(_) => None,
    }
}

/// `E6-01` at the first dataset, among those `plan` scans in the order its steps are written
/// (E6-4.12), with a row policy or a field the plan reads that needs such a value; then at
/// the origin of the first node that returns a column of such a type.
fn check_plan(plan: &PlanNode, needs: &Needs<'_>) -> Result<(), GenerateError> {
    let mut nodes = Vec::new();
    post_order(plan, &mut nodes);
    for node in &nodes {
        let Operation::Scan { dataset, .. } = node.operation() else {
            continue;
        };
        let Some(dataset) = needs.model.dataset(dataset) else {
            continue;
        };
        if dataset_needs(needs, dataset, node) {
            return Err(GenerateError::InexpressibleType {
                origin: Origin::Model(dataset.location.clone()),
                subject: Some(format!("dataset:{}", dataset.name.as_str())),
            });
        }
    }
    let returning = nodes.iter().find(|node| {
        node.schema()
            .columns()
            .iter()
            .any(|column| is_inexpressible_value(&column.logical_type, column.nullable))
    });
    match returning {
        Some(node) => Err(GenerateError::InexpressibleType {
            origin: node.origin().clone(),
            subject: None,
        }),
        None => Ok(()),
    }
}

/// Whether the `Scan` `node` of `dataset` needs such a value: a row policy of the dataset, or
/// a field it reads.
fn dataset_needs(needs: &Needs<'_>, dataset: &Dataset, node: &PlanNode) -> bool {
    let scope = Scope::Fields(vec![dataset]);
    let policies = dataset.row_policies.iter().any(|policy| {
        policy
            .condition
            .as_ref()
            .is_some_and(|condition| needs.expression(condition, &scope, false, 0))
    });
    policies
        || node.schema().columns().iter().any(|column| {
            dataset
                .fields
                .iter()
                .find(|field| field.name.as_str() == column.name.as_str())
                .is_some_and(|field| needs.field(dataset, field, 0))
        })
}

/// The nodes of `plan` in depth-first post-order, inputs in operand order (E6-4.12).
fn post_order<'plan>(plan: &'plan PlanNode, nodes: &mut Vec<&'plan PlanNode>) {
    for input in plan.inputs() {
        post_order(input, nodes);
    }
    nodes.push(plan);
}
