//! `EQRY-10` (QRY-4.5, E2-5.5) and `EQRY-04` (QRY-4.6): the ordering terms and the limit.

use std::collections::BTreeSet;

use menura_representations::bound_query::{BoundDimension, BoundMetric, BoundOrderItem};
use menura_representations::columns::{Columns, OutputColumn};
use menura_representations::names::OutputName;
use menura_representations::query_tree::{QueryLimit, QueryOrderItem};

use crate::error::{BindCode, BindDiagnostic};
use crate::names::output_key;

/// `EQRY-10`: each ordering term resolved to the column whose output name it matches after
/// the upper-case mapping (E2-5.5); the first term that matches none is reported.
pub(crate) fn bind_order(
    columns: &Columns<BoundDimension, BoundMetric>,
    order: &[QueryOrderItem],
) -> Result<Vec<BoundOrderItem>, BindDiagnostic> {
    let keys: Vec<String> = columns
        .iter()
        .map(|column| output_key(output_name(column)))
        .collect();
    order
        .iter()
        .map(|item| {
            let key = output_key(&item.output);
            let position = keys.iter().position(|candidate| *candidate == key);
            match position {
                Some(column) => Ok(BoundOrderItem {
                    column,
                    direction: item.direction,
                    nulls: item.nulls,
                    origin: item.origin.clone(),
                }),
                None => Err(BindDiagnostic::new(
                    BindCode::UnknownOrderOutput,
                    format!(
                        "`{}` is not an output name of the query; order by a dimension or a \
                         metric of the query",
                        item.output.as_str()
                    ),
                    &item.origin,
                )),
            }
        })
        .collect()
}

/// `EQRY-04`: a limit needs an ordering that includes every dimension output.
pub(crate) fn check_limit(
    columns: &Columns<BoundDimension, BoundMetric>,
    order: &[BoundOrderItem],
    limit: Option<&QueryLimit>,
) -> Result<(), BindDiagnostic> {
    let Some(limit) = limit else {
        return Ok(());
    };
    let ordered: BTreeSet<usize> = order.iter().map(|item| item.column).collect();
    let total = columns.iter().enumerate().all(|(position, column)| {
        matches!(column, OutputColumn::Metric(_)) || ordered.contains(&position)
    });
    if total {
        return Ok(());
    }
    Err(BindDiagnostic::new(
        BindCode::LimitWithoutTotalOrder,
        "a `limit` needs an `order_by` that includes every dimension of the query".to_owned(),
        &limit.origin,
    ))
}

fn output_name(column: &OutputColumn<BoundDimension, BoundMetric>) -> &OutputName {
    match column {
        OutputColumn::Dimension(dimension) => &dimension.output_name,
        OutputColumn::Metric(metric) => &metric.output_name,
    }
}
