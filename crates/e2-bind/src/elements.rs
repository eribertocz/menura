//! Row 3 of E2-5 over every element of the query, in query order (E2-5.3): the dimensions and
//! metrics, the filters, the date range, then the paths.

use menura_contracts::types::common::Grain;
use menura_model::expression::{Expression, Reference};
use menura_representations::bound_query::BoundPath;
use menura_representations::columns::OutputColumn;
use menura_representations::names::OutputName;
use menura_representations::origin::QueryLocation;
use menura_representations::query_tree::{
    QueryDateRange, QueryDimension, QueryDimensionKind, QueryFilter, QueryMetric, QueryMetricKind,
    QueryPath, QueryReference, QueryTree,
};

use crate::authorized_query::{AuthorizedDateRange, ResolvedName};
use crate::error::{BindCode, BindDiagnostic, BindError, InvariantViolation};
use crate::names::{written_expression_reference, written_reference};
use crate::resolver::Resolver;
use crate::walk::nodes;

/// Whether an output column is a dimension or a metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnRole {
    /// A dimension.
    Dimension,
    /// A metric.
    Metric,
}

/// An output column whose reference is resolved and authorized.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedColumn<'tree> {
    pub(crate) role: ColumnRole,
    pub(crate) reference: &'tree QueryReference,
    pub(crate) target: ResolvedName,
    pub(crate) time_grain: Option<Grain>,
    pub(crate) output_name: &'tree OutputName,
    pub(crate) origin: &'tree QueryLocation,
}

/// A filter whose references are resolved and authorized, in written order.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedFilter<'tree> {
    pub(crate) filter: &'tree QueryFilter,
    pub(crate) targets: Vec<ResolvedName>,
}

/// Every element of the query after row 3.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedElements<'tree> {
    pub(crate) columns: Vec<ResolvedColumn<'tree>>,
    pub(crate) filters: Vec<ResolvedFilter<'tree>>,
    pub(crate) date_range: Option<AuthorizedDateRange>,
    pub(crate) paths: Vec<BoundPath>,
}

/// Row 3 over every element of `tree`, in query order: the first name that does not resolve
/// or is not authorized is `E_NAME_NOT_FOUND`, located at its element.
pub(crate) fn resolve_elements<'tree>(
    tree: &'tree QueryTree,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedElements<'tree>, BindError> {
    let mut columns = Vec::new();
    for column in tree.columns().iter() {
        columns.push(match column {
            OutputColumn::Dimension(dimension) => resolve_dimension(dimension, resolver)?,
            OutputColumn::Metric(metric) => resolve_metric(metric, resolver)?,
        });
    }
    let mut filters = Vec::new();
    for filter in tree.filters() {
        filters.push(resolve_filter(filter, resolver)?);
    }
    let date_range = match tree.date_range() {
        Some(range) => Some(resolve_date_range(range, resolver)?),
        None => None,
    };
    let mut paths = Vec::new();
    for path in tree.paths() {
        paths.push(resolve_path(path, resolver)?);
    }
    Ok(ResolvedElements {
        columns,
        filters,
        date_range,
        paths,
    })
}

fn resolve_dimension<'tree>(
    dimension: &'tree QueryDimension,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedColumn<'tree>, BindError> {
    let QueryDimensionKind::Field {
        reference,
        time_grain,
    } = &dimension.kind
    else {
        return Err(BindError::invariant(
            InvariantViolation::SqlFormElement,
            &dimension.origin,
        ));
    };
    Ok(ResolvedColumn {
        role: ColumnRole::Dimension,
        reference,
        target: resolve(reference, &dimension.origin, resolver)?,
        time_grain: time_grain.as_ref().map(|choice| choice.grain),
        output_name: &dimension.output_name,
        origin: &dimension.origin,
    })
}

fn resolve_metric<'tree>(
    metric: &'tree QueryMetric,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedColumn<'tree>, BindError> {
    let QueryMetricKind::Reference(reference) = &metric.kind else {
        return Err(BindError::invariant(
            InvariantViolation::SqlFormElement,
            &metric.origin,
        ));
    };
    Ok(ResolvedColumn {
        role: ColumnRole::Metric,
        reference,
        target: resolve(reference, &metric.origin, resolver)?,
        time_grain: None,
        output_name: &metric.output_name,
        origin: &metric.origin,
    })
}

/// The references of a filter, in the order they appear in its text (E2-5.3).
fn resolve_filter<'tree>(
    filter: &'tree QueryFilter,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedFilter<'tree>, BindError> {
    let mut targets = Vec::new();
    for reference in nodes(&filter.expression)
        .into_iter()
        .filter_map(reference_of)
    {
        let target = resolver.expression_reference(reference).ok_or_else(|| {
            not_found(
                &written_expression_reference(reference),
                "field or metric",
                &filter.origin,
            )
        })?;
        targets.push(target);
    }
    Ok(ResolvedFilter { filter, targets })
}

/// The reference a node names: a field or metric reference, or the metric of `AGG(…)`.
fn reference_of(node: &Expression) -> Option<&Reference> {
    match node {
        Expression::Reference(reference) | Expression::MetricAggregate(reference) => {
            Some(reference)
        }
        _ => None,
    }
}

fn resolve_date_range(
    range: &QueryDateRange,
    resolver: &mut Resolver<'_>,
) -> Result<AuthorizedDateRange, BindError> {
    Ok(AuthorizedDateRange {
        target: resolve(&range.dimension, &range.origin, resolver)?,
        bounds: range.bounds.clone(),
        origin: range.origin.clone(),
    })
}

/// A path entry: `from`, `to`, then each relationship in order (E2-5.3).
fn resolve_path(path: &QueryPath, resolver: &mut Resolver<'_>) -> Result<BoundPath, BindError> {
    let origin = path.origin();
    let mut dataset = |reference: &QueryReference| {
        resolver
            .dataset_reference(reference)
            .ok_or_else(|| not_found(&written_reference(reference), "dataset", origin))
    };
    let from = dataset(path.from())?;
    let to = dataset(path.to())?;
    let mut relationships = Vec::new();
    for reference in path.relationships() {
        let relationship = resolver
            .relationship_reference(reference)
            .ok_or_else(|| not_found(&written_reference(reference), "relationship", origin))?;
        relationships.push(relationship);
    }
    BoundPath::new(from, to, relationships, origin.clone())
        .ok_or_else(|| BindError::invariant(InvariantViolation::PathWithoutRelationship, origin))
}

/// The field or metric `reference` names, or `E_NAME_NOT_FOUND` at `origin`.
fn resolve(
    reference: &QueryReference,
    origin: &QueryLocation,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedName, BindError> {
    resolver
        .reference(reference)
        .ok_or_else(|| not_found(&written_reference(reference), "field or metric", origin))
}

/// `E_NAME_NOT_FOUND` for the name `written`, which should name a `kind`. The message is the
/// same whether the name is unknown or unauthorized (E2-4.4, PLN-5.2).
fn not_found(written: &str, kind: &str, origin: &QueryLocation) -> BindError {
    let message = format!(
        "`{written}` does not name a {kind} of the model; check the name and how it is written"
    );
    BindDiagnostic::new(BindCode::NameNotFound, message, origin).into()
}
