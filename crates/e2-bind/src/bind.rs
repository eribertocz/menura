//! [`bind`]: E2 end to end, from the query tree to the bound query (E2-2).

use std::collections::BTreeMap;

use menura_contracts::types::common::Grain;
use menura_model::expression::DateValue;
use menura_model::internal_model::{FieldPath, InternalModel};
use menura_representations::bound_query::{
    BoundDimension, BoundDimensionKind, BoundMetric, BoundQuery, BoundQueryParts,
};
use menura_representations::columns::{Columns, OutputColumn};
use menura_representations::query_tree::QueryTree;
use menura_security::AdmittedSecurityContext;

use crate::authorized_query::AuthorizedQuery;
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
/// E0 (rows 10 to 12a); then `E0F-06`, `E2-01`, `EQRY-09`, `EQRY-10`, `EQRY-11`, `EQRY-12`,
/// `EQRY-03` and `EQRY-04` (row 12b); then `E_NOT_YET_SUPPORTED` for the first element outside
/// the 0.3.0 slice (row 12c, VS-1). [`BindError::Invariant`] when the tree breaks the contract
/// of E1.
pub fn bind(
    tree: &QueryTree,
    model: &InternalModel,
    context: &AdmittedSecurityContext,
) -> Result<BoundQuery, BindError> {
    let query = resolve_and_authorize(tree, model, context)?;
    check_rows_10_to_12a(model, &query.named)?;
    let bindings = check_row_12b(model, tree, &query)?;
    check_slice(model, &query.named)?;
    let reference_date = reference_date(tree);
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
    let columns = with_grains(&query.columns, &bindings.grains);
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

/// `columns` with the grain of each dimension's field (QRY-4.1).
fn with_grains(
    columns: &Columns<BoundDimension, BoundMetric>,
    grains: &BTreeMap<FieldPath, Grain>,
) -> Columns<BoundDimension, BoundMetric> {
    let columns = columns
        .iter()
        .cloned()
        .map(|column| match column {
            OutputColumn::Dimension(mut dimension) => {
                if let BoundDimensionKind::Field { field, time_grain } = &mut dimension.kind {
                    *time_grain = grains.get(field).copied();
                }
                OutputColumn::Dimension(dimension)
            }
            OutputColumn::Metric(metric) => OutputColumn::Metric(metric),
        })
        .collect();
    Columns::in_select_order(columns)
}

fn reference_date(_tree: &QueryTree) -> Option<DateValue> {
    None // PENDING #234 question 1
}
