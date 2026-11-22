//! Row 3 of E2-5 over every element of the query, in query order (E2-5.3): the `FROM` relation
//! of the SQL form, the dimensions and metrics, the filters, the time grains, the date range,
//! then the paths.

use menura_model::expression::{Expression, Reference};
use menura_representations::bound_query::BoundPath;
use menura_representations::columns::OutputColumn;
use menura_representations::names::OutputName;
use menura_representations::origin::QueryLocation;
use menura_representations::query_tree::{
    QueryDateRange, QueryDimension, QueryDimensionKind, QueryFilter, QueryMetric, QueryMetricKind,
    QueryPath, QueryReference, QueryRelation, QueryTree, TimeGrainChoice,
};

use crate::alternatives::ExpectedKind;
use crate::authorized_query::{AuthorizedDateRange, AuthorizedTimeGrain, ResolvedName};
use crate::error::{BindCode, BindDiagnostic, BindError, InvariantViolation};
use crate::names::{
    normalized_expression_reference, normalized_reference, written_expression_reference,
    written_identifier, written_reference,
};
use crate::resolver::{NameScope, RelationScope, Resolver};
use crate::walk::nodes;

/// Whether an output column is a dimension or a metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColumnRole {
    /// A dimension.
    Dimension,
    /// A metric.
    Metric,
}

/// One name an element reads, resolved and authorized.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedRead {
    /// The name as written, for messages.
    pub(crate) written: String,
    /// The model element it names.
    pub(crate) target: ResolvedName,
}

/// An output column whose names are resolved and authorized.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedColumn<'tree> {
    pub(crate) role: ColumnRole,
    /// SQL form: the expression of an expression dimension or of an inline metric; `None`
    /// for a column that is one reference.
    pub(crate) expression: Option<&'tree Expression>,
    /// The names it reads, in written order: one for a reference.
    pub(crate) reads: Vec<ResolvedRead>,
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
    /// SQL form: the `FROM` relation; `None` for the wide-table form.
    pub(crate) relation: Option<ResolvedRelation<'tree>>,
    pub(crate) columns: Vec<ResolvedColumn<'tree>>,
    pub(crate) filters: Vec<ResolvedFilter<'tree>>,
    pub(crate) time_grains: Vec<AuthorizedTimeGrain>,
    pub(crate) date_range: Option<AuthorizedDateRange>,
    pub(crate) paths: Vec<BoundPath>,
}

/// SQL form: the `FROM` relation, resolved and authorized.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedRelation<'tree> {
    pub(crate) scope: RelationScope,
    pub(crate) relation: &'tree QueryRelation,
}

/// Row 3 over every element of `tree`, in query order (E2-5.3): the `FROM` relation of the
/// SQL form, the columns, the filters, the time grains, the date range, then the paths. The
/// first name that does not resolve or is not authorized is `E_NAME_NOT_FOUND`, located at its
/// element.
pub(crate) fn resolve_elements<'tree>(
    tree: &'tree QueryTree,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedElements<'tree>, BindError> {
    let relation = match tree.relation() {
        Some(relation) => Some(resolve_relation(relation, resolver)?),
        None => None,
    };
    let scope = relation.as_ref().map_or(NameScope::Model, |resolved| {
        NameScope::Relation(&resolved.scope)
    });
    let mut columns = Vec::new();
    for column in tree.columns().iter() {
        columns.push(match column {
            OutputColumn::Dimension(dimension) => resolve_dimension(dimension, scope, resolver)?,
            OutputColumn::Metric(metric) => resolve_metric(metric, scope, resolver)?,
        });
    }
    let mut filters = Vec::new();
    for filter in tree.filters() {
        filters.push(resolve_filter(filter, scope, resolver)?);
    }
    let mut time_grains = Vec::new();
    for grain in tree.time_grains() {
        time_grains.push(resolve_time_grain(grain, resolver)?);
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
        relation,
        columns,
        filters,
        time_grains,
        date_range,
        paths,
    })
}

/// The `FROM` relation: its model is the model and its dataset an authorized dataset
/// (QRY-5.1, E1-4.9).
fn resolve_relation<'tree>(
    relation: &'tree QueryRelation,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedRelation<'tree>, BindError> {
    let scope = resolver.relation(relation).ok_or_else(|| {
        let written = format!(
            "{}.{}",
            written_identifier(&relation.model),
            written_identifier(&relation.dataset)
        );
        not_found(&written, "dataset", &relation.origin, Vec::new())
    })?;
    Ok(ResolvedRelation { scope, relation })
}

fn resolve_dimension<'tree>(
    dimension: &'tree QueryDimension,
    scope: NameScope<'_>,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedColumn<'tree>, BindError> {
    let (expression, reads) = match &dimension.kind {
        QueryDimensionKind::Field { reference } => (
            None,
            vec![resolve_read(
                reference,
                ExpectedKind::Field,
                scope,
                &dimension.origin,
                resolver,
            )?],
        ),
        QueryDimensionKind::Expression(expression) => (
            Some(expression),
            expression_reads(expression, scope, &dimension.origin, resolver)?,
        ),
    };
    Ok(ResolvedColumn {
        role: ColumnRole::Dimension,
        expression,
        reads,
        output_name: &dimension.output_name,
        origin: &dimension.origin,
    })
}

fn resolve_metric<'tree>(
    metric: &'tree QueryMetric,
    scope: NameScope<'_>,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedColumn<'tree>, BindError> {
    let (expression, reads) = match &metric.kind {
        QueryMetricKind::Reference(reference) => (
            None,
            vec![resolve_read(
                reference,
                ExpectedKind::Metric,
                scope,
                &metric.origin,
                resolver,
            )?],
        ),
        QueryMetricKind::Inline(expression) => (
            Some(expression),
            expression_reads(expression, scope, &metric.origin, resolver)?,
        ),
    };
    Ok(ResolvedColumn {
        role: ColumnRole::Metric,
        expression,
        reads,
        output_name: &metric.output_name,
        origin: &metric.origin,
    })
}

/// The column reference `reference`, resolved in `scope`, at a position that expects
/// `expected`.
fn resolve_read(
    reference: &QueryReference,
    expected: ExpectedKind,
    scope: NameScope<'_>,
    origin: &QueryLocation,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedRead, BindError> {
    let written = written_reference(reference);
    let target = resolver.scoped_reference(scope, reference).ok_or_else(|| {
        let alternatives =
            resolver.alternatives(expected, normalized_reference(reference).as_deref());
        not_found(&written, "field or metric", origin, alternatives)
    })?;
    Ok(ResolvedRead { written, target })
}

/// SQL form: the references of a column expression, in the order they appear in its text
/// (E2-5.3). The wide-table form has no column expressions (E1-5).
fn expression_reads(
    expression: &Expression,
    scope: NameScope<'_>,
    origin: &QueryLocation,
    resolver: &mut Resolver<'_>,
) -> Result<Vec<ResolvedRead>, BindError> {
    if matches!(scope, NameScope::Model) {
        return Err(BindError::invariant(
            InvariantViolation::SqlFormElement,
            origin,
        ));
    }
    let mut reads = Vec::new();
    for (reference, expected) in nodes(expression).into_iter().filter_map(expected_reference) {
        let written = written_expression_reference(reference);
        let target = resolver
            .scoped_expression_reference(scope, reference)
            .ok_or_else(|| expression_not_found(reference, expected, origin, resolver))?;
        reads.push(ResolvedRead { written, target });
    }
    Ok(reads)
}

/// The references of a filter, in the order they appear in its text (E2-5.3).
fn resolve_filter<'tree>(
    filter: &'tree QueryFilter,
    scope: NameScope<'_>,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedFilter<'tree>, BindError> {
    let mut targets = Vec::new();
    for (reference, expected) in nodes(&filter.expression)
        .into_iter()
        .filter_map(expected_reference)
    {
        let target = resolver
            .scoped_expression_reference(scope, reference)
            .ok_or_else(|| expression_not_found(reference, expected, &filter.origin, resolver))?;
        targets.push(target);
    }
    Ok(ResolvedFilter { filter, targets })
}

/// The reference a node names: a field or metric reference, or the metric of `AGG(…)`.
pub(crate) fn reference_of(node: &Expression) -> Option<&Reference> {
    match node {
        Expression::Reference(reference) | Expression::MetricAggregate(reference) => {
            Some(reference)
        }
        _ => None,
    }
}

/// The reference a node names, with the kind its position expects (PLN-5.4 step 1): a field
/// for a field or metric reference, a metric for the metric of `AGG(…)`.
fn expected_reference(node: &Expression) -> Option<(&Reference, ExpectedKind)> {
    match node {
        Expression::Reference(reference) => Some((reference, ExpectedKind::Field)),
        Expression::MetricAggregate(reference) => Some((reference, ExpectedKind::Metric)),
        _ => None,
    }
}

/// `E_NAME_NOT_FOUND` for the expression reference `reference`, at a position that expects
/// `expected`, with its alternatives (PLN-5.4).
fn expression_not_found(
    reference: &Reference,
    expected: ExpectedKind,
    origin: &QueryLocation,
    resolver: &Resolver<'_>,
) -> BindError {
    let normalized = normalized_expression_reference(reference);
    let alternatives = resolver.alternatives(expected, Some(&normalized));
    not_found(
        &written_expression_reference(reference),
        "field or metric",
        origin,
        alternatives,
    )
}

fn resolve_time_grain(
    grain: &TimeGrainChoice,
    resolver: &mut Resolver<'_>,
) -> Result<AuthorizedTimeGrain, BindError> {
    Ok(AuthorizedTimeGrain {
        target: resolve(&grain.dimension, &grain.origin, resolver)?,
        grain: grain.grain,
        origin: grain.origin.clone(),
    })
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
        resolver.dataset_reference(reference).ok_or_else(|| {
            let alternatives = resolver.alternatives(
                ExpectedKind::Dataset,
                normalized_reference(reference).as_deref(),
            );
            not_found(
                &written_reference(reference),
                "dataset",
                origin,
                alternatives,
            )
        })
    };
    let from = dataset(path.from())?;
    let to = dataset(path.to())?;
    let mut relationships = Vec::new();
    for reference in path.relationships() {
        let relationship = resolver.relationship_reference(reference).ok_or_else(|| {
            not_found(
                &written_reference(reference),
                "relationship",
                origin,
                Vec::new(),
            )
        })?;
        relationships.push(relationship);
    }
    BoundPath::new(from, to, relationships, origin.clone())
        .ok_or_else(|| BindError::invariant(InvariantViolation::PathWithoutRelationship, origin))
}

/// The field `reference` names, or `E_NAME_NOT_FOUND` at `origin` with the fields closest to
/// it (PLN-5.4).
fn resolve(
    reference: &QueryReference,
    origin: &QueryLocation,
    resolver: &mut Resolver<'_>,
) -> Result<ResolvedName, BindError> {
    resolver.reference(reference).ok_or_else(|| {
        let alternatives = resolver.alternatives(
            ExpectedKind::Field,
            normalized_reference(reference).as_deref(),
        );
        not_found(
            &written_reference(reference),
            "field or metric",
            origin,
            alternatives,
        )
    })
}

/// `E_NAME_NOT_FOUND` for the name `written`, which should name a `kind`, carrying
/// `alternatives` (PLN-5.4). The message and the alternatives are the same whether the name is
/// unknown or unauthorized (E2-4.4, PLN-5.2, PLN-5.4a).
fn not_found(
    written: &str,
    kind: &str,
    origin: &QueryLocation,
    alternatives: Vec<String>,
) -> BindError {
    let message = format!(
        "`{written}` does not name a {kind} of the model; check the name and how it is written"
    );
    BindDiagnostic::new(BindCode::NameNotFound, message, origin)
        .with_alternatives(alternatives)
        .into()
}
