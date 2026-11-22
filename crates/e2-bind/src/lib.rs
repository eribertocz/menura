//! Stage E2: binds query references to the loaded model.
//!
//! [`resolve_and_authorize`] runs rows 3 to 9 of the error precedence of E2-5 on a query tree:
//! every name resolves against the internal model (E2-3) and is authorized against the
//! admitted security context as soon as it resolves (E2-4). [`bind`] runs it, then rows 10 to
//! 12c, and builds the bound query (E2-2). A SQL-form query is bound like a wide-table query,
//! with its names read through its `FROM` relation (QRY-5).

#![forbid(unsafe_code)]

mod authorized_query;
mod bind;
mod calendar;
mod canonical;
mod elements;
mod error;
mod model_elements;
mod named;
mod names;
mod read_metrics;
mod resolver;
mod row_12b;
mod rows;
mod slice;
mod unusable;
mod walk;

pub use authorized_query::{
    AuthorizedDateRange, AuthorizedQuery, AuthorizedTimeGrain, ResolvedName,
};
pub use bind::bind;
pub use error::{BindCode, BindDiagnostic, BindError, InvariantViolation};

use menura_model::internal_model::{InternalModel, NormalizedName};
use menura_representations::bound_query::{
    AuthorizationDecision, AuthorizedSubgraph, BoundDimension, BoundDimensionKind, BoundFilter,
    BoundMetric, BoundMetricKind,
};
use menura_representations::columns::{Columns, OutputColumn};
use menura_representations::query_tree::QueryTree;
use menura_security::AdmittedSecurityContext;

use canonical::canonical;
use elements::{ColumnRole, ResolvedColumn, ResolvedElements, ResolvedRead, resolve_elements};
use named::{NamedElement, named_elements};
use resolver::Resolver;
use rows::{check_rows_4_to_9, filter_datasets};

/// Resolves and authorizes every name of `tree` against `model` for the user of `context`,
/// then checks rows 4 to 9 of E2-5.
///
/// # Errors
///
/// [`BindError::Diagnostic`] with the first error in the order of E2-5 rows 3 to 9, and within
/// a row the first element in query order (E2-5.3): `E_NAME_NOT_FOUND` for a name that is
/// unknown or unauthorized, both reported alike (E2-4.4); `E_METRIC_IN_DIMENSIONS`;
/// `E_FIELD_IN_METRICS`; `E_AGGREGATE_IN_WHERE`; `E_CROSS_DATASET_FILTER`; `E_NAME_COLLISION`.
/// [`BindError::Invariant`] when the tree breaks the contract of E1.
pub fn resolve_and_authorize(
    tree: &QueryTree,
    model: &InternalModel,
    context: &AdmittedSecurityContext,
) -> Result<AuthorizedQuery, BindError> {
    let mut resolver = Resolver::new(model, context);
    let elements = resolve_elements(tree, &mut resolver)?;
    check_rows_4_to_9(&elements)?;
    let named = named_elements(tree, &elements);
    let (authorization, subgraph) = resolver.finish();
    Ok(assemble(elements, named, authorization, subgraph))
}

/// The authorized query of `elements`, which passed rows 3 to 9. In the SQL form, every
/// expression is written with the model elements its references resolved to
/// ([`canonical::canonical`]).
fn assemble(
    elements: ResolvedElements<'_>,
    named: Vec<NamedElement>,
    authorization: Vec<AuthorizationDecision>,
    subgraph: AuthorizedSubgraph,
) -> AuthorizedQuery {
    let relation = elements
        .relation
        .as_ref()
        .map(|resolved| resolved.scope.dataset());
    let filters = elements
        .filters
        .iter()
        .map(|filter| BoundFilter {
            expression: match relation {
                Some(_) => canonical(&filter.filter.expression, &filter.targets),
                None => filter.filter.expression.clone(),
            },
            datasets: filter_datasets(filter),
            origin: filter.filter.origin.clone(),
        })
        .collect();
    let columns = elements
        .columns
        .iter()
        .filter_map(|column| bound_column(column, relation))
        .collect();
    AuthorizedQuery {
        columns: Columns::in_select_order(columns),
        filters,
        time_grains: elements.time_grains,
        date_range: elements.date_range,
        paths: elements.paths,
        authorization,
        subgraph,
        named,
    }
}

/// The bound column of `column`; rows 4 and 5 made every dimension read fields only and every
/// metric read metrics only. `relation` is the dataset of the SQL form's `FROM` relation.
fn bound_column(
    column: &ResolvedColumn<'_>,
    relation: Option<&NormalizedName>,
) -> Option<OutputColumn<BoundDimension, BoundMetric>> {
    let output_name = column.output_name.clone();
    let origin = column.origin.clone();
    Some(match column.role {
        ColumnRole::Dimension => OutputColumn::Dimension(BoundDimension {
            kind: dimension_kind(column, relation)?,
            output_name,
            origin,
        }),
        ColumnRole::Metric => OutputColumn::Metric(BoundMetric {
            kind: metric_kind(column)?,
            output_name,
            origin,
        }),
    })
}

/// What the dimension `column` reads: its field, or its expression over fields of `relation`.
fn dimension_kind(
    column: &ResolvedColumn<'_>,
    relation: Option<&NormalizedName>,
) -> Option<BoundDimensionKind> {
    match (column.expression, column.reads.as_slice()) {
        (None, [read]) => match &read.target {
            ResolvedName::Field(field) => Some(BoundDimensionKind::Field {
                field: field.clone(),
                time_grain: None,
                time_grain_origin: None,
            }),
            ResolvedName::Metric(_) => None,
        },
        (Some(expression), reads) => Some(BoundDimensionKind::Expression {
            dataset: relation?.clone(),
            expression: canonical(expression, &targets(reads)),
        }),
        (None, _) => None,
    }
}

/// What the metric `column` reads: its metric, or its expression over metrics.
fn metric_kind(column: &ResolvedColumn<'_>) -> Option<BoundMetricKind> {
    match (column.expression, column.reads.as_slice()) {
        (None, [read]) => match &read.target {
            ResolvedName::Metric(metric) => Some(BoundMetricKind::Reference(metric.clone())),
            ResolvedName::Field(_) => None,
        },
        (Some(expression), reads) => Some(BoundMetricKind::Inline(canonical(
            expression,
            &targets(reads),
        ))),
        (None, _) => None,
    }
}

/// The targets of `reads`, in order.
fn targets(reads: &[ResolvedRead]) -> Vec<ResolvedName> {
    reads.iter().map(|read| read.target.clone()).collect()
}
