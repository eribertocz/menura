//! [`bind`]: E2 end to end, from the query tree to the bound query (E2-2).

use std::collections::BTreeMap;

use menura_contracts::types::common::Grain;
use menura_model::expression::DateValue;
use menura_model::internal_model::{FieldPath, InternalModel};
use menura_representations::bound_query::{
    BoundDimension, BoundDimensionKind, BoundMetric, BoundQuery, BoundQueryParts,
};
use menura_representations::columns::{Columns, OutputColumn};
use menura_representations::origin::QueryLocation;
use menura_representations::query_tree::QueryTree;
use menura_security::AdmittedSecurityContext;

use crate::authorized_query::{AuthorizedQuery, ResolvedName};
use crate::error::BindError;
use crate::resolve_and_authorize;
use crate::row_12b::{Row12bBindings, check_row_12b, resolve_date_range};
use crate::slice::check_slice;
use crate::unusable::check_rows_10_to_12a;

/// Binds `tree` to `model` for the user of `context`: rows 3 to 9 of E2-5
/// ([`resolve_and_authorize`]), then rows 10 to 12c, and builds the bound query.
///
/// # Errors
///
/// [`BindError::Diagnostic`] with the first error in the order of E2-5 rows 3 to 12c, and
/// within a row the first element in query order (E2-5.3): the errors of
/// [`resolve_and_authorize`]; then the query error code of a named element marked unusable in
/// E0 (rows 10 to 12a); then the E0.f code of a filter that does not type or is not boolean
/// (`E0F-03` to `E0F-08`), `E2-01`, `EQRY-09`, `EQRY-10`, `EQRY-11`, `EQRY-12`,
/// `EQRY-03`, `E_INVALID_QUERY` and `EQRY-04` (row 12b); then `E_NOT_YET_SUPPORTED` for the first element outside
/// the 0.3.0 slice (row 12c, VS-1). [`BindError::Invariant`] when the tree breaks the contract
/// of E1.
pub fn bind(
    tree: &QueryTree,
    model: &InternalModel,
    context: &AdmittedSecurityContext,
) -> Result<BoundQuery, BindError> {
    let query = resolve_and_authorize(tree, model, context)?;
    let reference_date = tree.time().reference_date;
    check_rows_10_to_12a(model, &query.named)?;
    let bindings = check_row_12b(model, tree, &query, reference_date)?;
    check_slice(model, &query.named)?;
    assemble(tree, query, bindings, reference_date)
}

/// The bound query of `query`, which passed rows 3 to 12c.
fn assemble(
    tree: &QueryTree,
    query: AuthorizedQuery,
    bindings: Row12bBindings,
    reference_date: Option<DateValue>,
) -> Result<BoundQuery, BindError> {
    let date_range = match &query.date_range {
        Some(range) => Some(resolve_date_range(range, reference_date)?),
        None => None,
    };
    let columns = with_grains(&query.columns, &bindings.grains, &grain_origins(&query));
    let parts = BoundQueryParts {
        filters: query.filters,
        date_range,
        paths: query.paths,
        order: bindings.order,
        limit: tree.limit().cloned(),
        authorization: query.authorization,
        subgraph: query.subgraph,
    };
    Ok(BoundQuery::new(columns, tree.time().clone(), parts))
}

/// Where the `time_grains` entry of each field is written. A field with two entries never gets
/// here (`EQRY-09`).
fn grain_origins(query: &AuthorizedQuery) -> BTreeMap<FieldPath, QueryLocation> {
    query
        .time_grains
        .iter()
        .filter_map(|grain| match &grain.target {
            ResolvedName::Field(field) => Some((field.clone(), grain.origin.clone())),
            ResolvedName::Metric(_) => None,
        })
        .collect()
}

/// `columns` with the grain of each dimension's field and where its entry is written
/// (QRY-4.1, E4-7.8).
fn with_grains(
    columns: &Columns<BoundDimension, BoundMetric>,
    grains: &BTreeMap<FieldPath, Grain>,
    origins: &BTreeMap<FieldPath, QueryLocation>,
) -> Columns<BoundDimension, BoundMetric> {
    let columns = columns
        .iter()
        .cloned()
        .map(|column| match column {
            OutputColumn::Dimension(mut dimension) => {
                if let BoundDimensionKind::Field {
                    field,
                    time_grain,
                    time_grain_origin,
                } = &mut dimension.kind
                {
                    *time_grain = grains.get(field).copied();
                    *time_grain_origin = origins.get(field).cloned();
                }
                OutputColumn::Dimension(dimension)
            }
            OutputColumn::Metric(metric) => OutputColumn::Metric(metric),
        })
        .collect();
    Columns::in_select_order(columns)
}
