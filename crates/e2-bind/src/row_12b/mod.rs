//! Row 12b of E2-5, in its order: `E0F-06`, `E2-01`, `EQRY-09`, `EQRY-10`, `EQRY-11`,
//! `EQRY-12`, `EQRY-03`, `E_INVALID_QUERY` (a relative range outside the years 0001–9999),
//! `EQRY-04`. Each check runs over every element before the next one.

mod date_range;
mod order;
mod paths;
mod time_fields;

use std::collections::BTreeMap;

use menura_contracts::types::common::Grain;
use menura_model::expression::DateValue;
use menura_model::internal_model::{FieldPath, InternalModel};
use menura_representations::bound_query::BoundOrderItem;
use menura_representations::query_tree::QueryTree;

use crate::authorized_query::AuthorizedQuery;
use crate::error::BindError;

pub(crate) use date_range::resolve_date_range;

/// What row 12b binds: the grain of each dimension field and the ordering terms.
pub(crate) struct Row12bBindings {
    pub(crate) grains: BTreeMap<FieldPath, Grain>,
    pub(crate) order: Vec<BoundOrderItem>,
}

/// Row 12b over `query`, with the ordering terms, the limit and the time of `tree`.
pub(crate) fn check_row_12b(
    model: &InternalModel,
    tree: &QueryTree,
    query: &AuthorizedQuery,
    reference_date: Option<DateValue>,
) -> Result<Row12bBindings, BindError> {
    time_fields::check_time_fields(model, query)?;
    let grains = time_fields::assign_time_grains(query)?;
    let order = order::bind_order(&query.columns, tree.order())?;
    date_range::check_order_of_bounds(query.date_range.as_ref())?;
    paths::check_paths(model, &query.paths)?;
    date_range::check_reference_time(
        &query.filters,
        query.date_range.as_ref(),
        tree.time().reference_time.is_some(),
    )?;
    date_range::check_range_years(query.date_range.as_ref(), reference_date)?;
    order::check_limit(&query.columns, &order, tree.limit())?;
    Ok(Row12bBindings { grains, order })
}
