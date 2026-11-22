//! Rows 4 to 9 of E2-5. Each row is checked over every element before the next row, and the
//! first element in query order that fails is reported, located at that element (E2-5.3).

use std::collections::BTreeSet;

use menura_model::expression::Expression;
use menura_model::internal_model::NormalizedName;

use crate::authorized_query::ResolvedName;
use crate::elements::{ColumnRole, ResolvedColumn, ResolvedElements, ResolvedFilter};
use crate::error::{BindCode, BindDiagnostic};
use crate::names::output_key;
use crate::walk::nodes;

/// Rows 4 to 9, in order, over the elements row 3 resolved.
pub(crate) fn check_rows_4_to_9(elements: &ResolvedElements<'_>) -> Result<(), BindDiagnostic> {
    let sql_form = elements.relation.is_some();
    metric_in_dimensions(&elements.columns, sql_form)?;
    field_in_metrics(&elements.columns, sql_form)?;
    aggregate_in_where(&elements.filters)?;
    // Row 7, `E_WINDOW_IN_WHERE`: no query tree holds a window function, because the
    // expression language rejects one as `E0F-02` (E0F-2.4), which E1 reports at row 1.
    cross_dataset_filter(&elements.filters)?;
    name_collision(&elements.columns)
}

/// The datasets whose fields `filter` reads, ordered by name without repeats.
pub(crate) fn filter_datasets(filter: &ResolvedFilter<'_>) -> Vec<NormalizedName> {
    let datasets: BTreeSet<&NormalizedName> = filter
        .targets
        .iter()
        .filter_map(|target| match target {
            ResolvedName::Field(field) => Some(&field.dataset),
            ResolvedName::Metric(_) => None,
        })
        .collect();
    datasets.into_iter().cloned().collect()
}

/// Row 4: a dimension MUST name a field, and an expression dimension of the SQL form MUST read
/// fields and no metric (E2-3.3, E1-4.9).
fn metric_in_dimensions(
    columns: &[ResolvedColumn<'_>],
    sql_form: bool,
) -> Result<(), BindDiagnostic> {
    for column in columns
        .iter()
        .filter(|column| column.role == ColumnRole::Dimension)
    {
        let metric = column
            .reads
            .iter()
            .find(|read| matches!(read.target, ResolvedName::Metric(_)));
        let message = match metric {
            Some(read) if sql_form => format!(
                "`{}` names a metric, which a `SELECT` item reads only inside `AGG(…)`; \
                 write `AGG({})`",
                read.written, read.written
            ),
            Some(read) => format!(
                "dimension `{}` names a metric; request it under `metrics`",
                read.written
            ),
            None if column.reads.is_empty() => format!(
                "`{}` reads no field; a `SELECT` item without `AGG(…)` reads fields of the \
                 relation",
                column.output_name.as_str()
            ),
            None => continue,
        };
        return Err(BindDiagnostic::new(
            BindCode::MetricInDimensions,
            message,
            column.origin,
        ));
    }
    Ok(())
}

/// Row 5: a metric MUST name a metric, and `AGG(…)` of the SQL form MUST take a metric
/// (E2-3.3, E1-4.9).
fn field_in_metrics(columns: &[ResolvedColumn<'_>], sql_form: bool) -> Result<(), BindDiagnostic> {
    for column in columns
        .iter()
        .filter(|column| column.role == ColumnRole::Metric)
    {
        let field = column
            .reads
            .iter()
            .find(|read| matches!(read.target, ResolvedName::Field(_)));
        if let Some(read) = field {
            let message = if sql_form {
                format!(
                    "`AGG({})` names a field; `AGG(…)` takes a metric of the relation, and a \
                     field is a `SELECT` item of its own",
                    read.written
                )
            } else {
                format!(
                    "metric `{}` names a field; request it under `dimensions`, or name a metric",
                    read.written
                )
            };
            return Err(BindDiagnostic::new(
                BindCode::FieldInMetrics,
                message,
                column.origin,
            ));
        }
    }
    Ok(())
}

/// Row 6: a filter holds no aggregate and no metric.
fn aggregate_in_where(filters: &[ResolvedFilter<'_>]) -> Result<(), BindDiagnostic> {
    let found = filters.iter().find(|filter| {
        let aggregates = nodes(&filter.filter.expression).into_iter().any(|node| {
            matches!(
                node,
                Expression::Aggregate { .. } | Expression::MetricAggregate(_)
            )
        });
        aggregates
            || filter
                .targets
                .iter()
                .any(|target| matches!(target, ResolvedName::Metric(_)))
    });
    match found {
        Some(filter) => Err(BindDiagnostic::new(
            BindCode::AggregateInWhere,
            "this filter holds an aggregate or a metric; a filter reads fields only".to_owned(),
            &filter.filter.origin,
        )),
        None => Ok(()),
    }
}

/// Row 8: a filter reads fields of one dataset at most.
fn cross_dataset_filter(filters: &[ResolvedFilter<'_>]) -> Result<(), BindDiagnostic> {
    match filters
        .iter()
        .find(|filter| filter_datasets(filter).len() > 1)
    {
        Some(filter) => Err(BindDiagnostic::new(
            BindCode::CrossDatasetFilter,
            "this filter reads fields of more than one dataset; write one filter per dataset"
                .to_owned(),
            &filter.filter.origin,
        )),
        None => Ok(()),
    }
}

/// Row 9: output names are distinct after the upper-case mapping (E2-5.5). The first column
/// whose name equals the name of an earlier column is reported.
fn name_collision(columns: &[ResolvedColumn<'_>]) -> Result<(), BindDiagnostic> {
    let mut seen = BTreeSet::new();
    for column in columns {
        if !seen.insert(output_key(column.output_name)) {
            return Err(BindDiagnostic::new(
                BindCode::NameCollision,
                format!(
                    "output name `{}` is already the name of an earlier column, ignoring case; \
                     give one of them another alias",
                    column.output_name.as_str()
                ),
                column.origin,
            ));
        }
    }
    Ok(())
}
