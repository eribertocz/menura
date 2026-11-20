//! Rows 4 to 9 of E2-5. Each row is checked over every element before the next row, and the
//! first element in query order that fails is reported, located at that element (E2-5.3).

use std::collections::BTreeSet;

use menura_model::expression::Expression;
use menura_model::internal_model::NormalizedName;

use crate::authorized_query::ResolvedName;
use crate::elements::{ColumnRole, ResolvedColumn, ResolvedElements, ResolvedFilter};
use crate::error::{BindCode, BindDiagnostic};
use crate::names::{output_key, written_reference};
use crate::walk::nodes;

/// Rows 4 to 9, in order, over the elements row 3 resolved.
pub(crate) fn check_rows_4_to_9(elements: &ResolvedElements<'_>) -> Result<(), BindDiagnostic> {
    metric_in_dimensions(&elements.columns)?;
    field_in_metrics(&elements.columns)?;
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

/// Row 4: a dimension MUST name a field (E2-3.3).
fn metric_in_dimensions(columns: &[ResolvedColumn<'_>]) -> Result<(), BindDiagnostic> {
    let found = columns.iter().find(|column| {
        column.role == ColumnRole::Dimension && matches!(column.target, ResolvedName::Metric(_))
    });
    match found {
        Some(column) => Err(BindDiagnostic::new(
            BindCode::MetricInDimensions,
            format!(
                "dimension `{}` names a metric; request it under `metrics`",
                written_reference(column.reference)
            ),
            column.origin,
        )),
        None => Ok(()),
    }
}

/// Row 5: a metric MUST name a metric (E2-3.3).
fn field_in_metrics(columns: &[ResolvedColumn<'_>]) -> Result<(), BindDiagnostic> {
    let found = columns.iter().find(|column| {
        column.role == ColumnRole::Metric && matches!(column.target, ResolvedName::Field(_))
    });
    match found {
        Some(column) => Err(BindDiagnostic::new(
            BindCode::FieldInMetrics,
            format!(
                "metric `{}` names a field; request it under `dimensions`, or name a metric",
                written_reference(column.reference)
            ),
            column.origin,
        )),
        None => Ok(()),
    }
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
