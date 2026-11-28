//! The checks of E6-7.1 and E6-7.12, before any SQL is written: a plan that scans a relation
//! `source` of three or more parts stops with `E6-01` at its dataset; then a plan that reads,
//! computes or returns a value `ClickHouse` cannot express stops with `E6-01`.
//!
//! The error is located at the first query element, in the order E2 checks them (E2-5.3),
//! whose evaluation needs such a value: the dimensions, the metrics, the `filters` items, then
//! `date_range`. When no query element needs one, the value is in a row policy or another
//! model expression, and the error is located at the dataset that holds it.

mod needs;
mod open_end;
mod sources;
mod walk;

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_model::internal_model::{Dataset, InternalModel, TypedValue, ValueType};
use menura_representations::bound_query::{
    BoundDimensionKind, BoundMetricKind, BoundQuery, MetricPath,
};
use menura_representations::columns::OutputColumn;
use menura_representations::logical_plan::{Operation, PlanNode};
use menura_representations::origin::{Origin, QueryLocation};

use crate::error::GenerateError;
use crate::types::is_inexpressible_value;
use needs::{Needs, Scope, after_nanosecond_range, is_nanosecond_timestamp, outside_date32};
use open_end::check_node_open_end;
pub(crate) use open_end::{check_open_end, check_rule_open_end};
use sources::check_sources;

/// `E6-01` when `plan`, the plan of `query` over `model` and `catalog`, scans a relation
/// `source` of three or more parts (E6-7.1), then when it needs a value `ClickHouse` cannot
/// express, located as E6-7.12 states.
///
/// # Errors
///
/// [`GenerateError::InexpressibleSource`] or [`GenerateError::InexpressibleType`] when it
/// does.
pub(crate) fn check_query(
    plan: &PlanNode,
    query: &BoundQuery,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<(), GenerateError> {
    check_sources(plan, model)?;
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

/// `E6-01` when one of `plans`, the plans an integrity-check statement writes, in the order
/// it writes them, scans a relation `source` of three or more parts (E6-7.1); then when one
/// of them, in the same order, needs such a value, located at the dataset that holds it
/// (E6-7.11, E6-7.12).
///
/// # Errors
///
/// [`GenerateError::InexpressibleSource`] or [`GenerateError::InexpressibleType`] when it
/// does.
pub(crate) fn check_model_plans(
    plans: &[&PlanNode],
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
) -> Result<(), GenerateError> {
    for plan in plans {
        check_sources(plan, model)?;
    }
    let needs = Needs {
        model,
        catalog,
        reference_date: None,
    };
    for plan in plans {
        check_plan(plan, &needs)?;
    }
    Ok(())
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
    let nanoseconds = field.is_some_and(|(_, field)| {
        matches!(&field.typed, Some(TypedValue { value_type: ValueType::Logical(logical_type), .. }) if is_nanosecond_timestamp(logical_type))
    });
    let outside = |date| outside_date32(date) || (nanoseconds && after_nanosecond_range(date));
    (field_needs || outside(range.from) || outside(range.to)).then(|| {
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
/// (E6-4.12), with a row policy or a field the plan reads that needs such a value, or that
/// declares a population whose `open_end` a node reads outside the range of its type
/// (VAL-2.23); then at the origin of the first node that returns a column of such a type.
fn check_plan(plan: &PlanNode, needs: &Needs<'_>) -> Result<(), GenerateError> {
    let mut nodes = Vec::new();
    post_order(plan, &mut nodes);
    for node in &nodes {
        check_node_open_end(node, needs.model)?;
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
