//! The query of the `semi_additive` check of a metric (CHK-6.5, ADD-6): every row of the
//! metric's home dataset (ADD-5.1.6), read by a `Scan` with no row policy (CHK-2.2), and with
//! a time `over` every row of the dataset of its population.

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::{Dialect, Grain, LogicalType};
use menura_generator_duckdb::{SnapshotCheck, SnapshotTime};
use menura_model::internal_model::{
    DeclaredAdditivity, FieldPath, InternalModel, Metric, NormalizedName, SemiAdditive,
};
use menura_representations::logical_plan::{PlanNode, PopulationRule};
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::key_columns::{KeyColumn, STAGE, key_columns};
use super::plan_nodes::scan;
use super::population_checks::{declared, expected_population_rows, population_rule};
use super::result_names::added_column_name;
use crate::compile::InternalError;
use crate::dialect_sql::{CheckShape, CheckSql, Sources, check_sql};
use crate::stop::Stop;

/// The SQL of the `semi_additive` check of the metric `target` in `dialect`, and the datasets
/// it reads: the home dataset, then, with a time `over`, the dataset of the population. `None`
/// when `over` or an `entity` field is not a field of the home dataset: that check is left
/// out.
pub(super) fn semi_additive_check_sql(
    dialect: Dialect,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
    target: &str,
) -> Result<Option<(CheckSql, Vec<NormalizedName>)>, Stop> {
    let (metric, keys) = semi_additive_metric(model, target)?;
    let Some(home_name) = home_dataset(metric, keys, target)? else {
        return Ok(None);
    };
    let home = model
        .dataset(home_name)
        .ok_or_else(|| defect(target, "names an unknown home dataset"))?;
    let read: Vec<NormalizedName> = keys
        .entity
        .iter()
        .chain([&keys.over])
        .map(|path| path.field.clone())
        .collect();
    let read_columns = key_columns(home, &read, catalog)?;
    let rows = scan(
        &home.name,
        &read_columns,
        &Origin::Model(metric.location.clone()),
    );
    let mut result_columns: Vec<String> =
        read.iter().map(|name| name.as_str().to_owned()).collect();
    result_columns.push(added_column_name("violation", &result_columns));
    let mut datasets = vec![home.name.clone()];
    let population_parts = time_parts(model, catalog, keys, &read_columns, target)?;
    if let Some(parts) = &population_parts {
        datasets.push(parts.dataset.clone());
    }
    let check = SnapshotCheck {
        rows: &rows,
        entity: keys
            .entity
            .iter()
            .map(|path| ColumnName::new(path.field.as_str()))
            .collect(),
        over: ColumnName::new(keys.over.field.as_str()),
        time: population_parts.as_ref().map(|parts| SnapshotTime {
            grain: parts.grain,
            population: &parts.rows,
            rule: &parts.rule,
        }),
        result_columns,
    };
    let generated = check_sql(
        dialect,
        &CheckShape::Snapshot(check),
        Sources { catalog, model },
    )?;
    Ok(Some((generated, datasets)))
}

/// The population part of a check with a time `over`.
struct TimeParts {
    grain: Grain,
    dataset: NormalizedName,
    rows: PlanNode,
    rule: PopulationRule,
}

/// With a time `over` (the last of `read_columns`, ADD-5.1.5), its `snapshot_grain` and the
/// rows and rule of its population; `None` for any other `over`.
fn time_parts(
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
    keys: &SemiAdditive,
    read_columns: &[KeyColumn],
    target: &str,
) -> Result<Option<TimeParts>, InternalError> {
    if !is_time(read_columns) {
        return Ok(None);
    }
    let (Some(grain), Some(population)) = (keys.snapshot_grain, &keys.population) else {
        return Err(defect(
            target,
            "has a time over without grain or population",
        ));
    };
    let (dataset, declared) = declared(model, population)?;
    Ok(Some(TimeParts {
        grain,
        dataset: dataset.name.clone(),
        rows: expected_population_rows(dataset, declared, catalog)?,
        rule: population_rule(declared),
    }))
}

/// The metric `target` (a model metric, or `<dataset>.<metric>` for a dataset metric,
/// E0H-4.3) and its `semi_additive` keys.
fn semi_additive_metric<'model>(
    model: &'model InternalModel,
    target: &str,
) -> Result<(&'model Metric, &'model SemiAdditive), InternalError> {
    let parts = model.parts();
    let metric = parts
        .metrics
        .iter()
        .find(|metric| metric.name.as_str() == target)
        .or_else(|| {
            parts.datasets.iter().find_map(|dataset| {
                dataset.metrics.iter().find(|metric| {
                    format!("{}.{}", dataset.name.as_str(), metric.name.as_str()) == target
                })
            })
        })
        .ok_or_else(|| defect(target, "names no metric"))?;
    match &metric.declared_additivity {
        Some(DeclaredAdditivity::SemiAdditive(keys)) => Ok((metric, keys)),
        _ => Err(defect(target, "names a metric that is not semi-additive")),
    }
}

/// The one home dataset of `metric` (ADD-5.1.6), or `None` when `over` or an `entity` field
/// is a field of a dataset reached from it: the check reads them as fields of the home
/// dataset only.
fn home_dataset<'model>(
    metric: &'model Metric,
    keys: &SemiAdditive,
    target: &str,
) -> Result<Option<&'model NormalizedName>, InternalError> {
    let homes = metric
        .classification
        .as_ref()
        .map(|classification| classification.home_datasets.as_slice())
        .unwrap_or_default();
    let [home] = homes else {
        return Err(defect(target, "has not exactly one home dataset"));
    };
    let on_home = |path: &FieldPath| &path.dataset == home;
    let read_on_home = on_home(&keys.over) && keys.entity.iter().all(on_home);
    Ok(read_on_home.then_some(home))
}

/// Whether the `over` column, the last of `columns`, is a time field (ADD-5.1.5).
fn is_time(columns: &[KeyColumn]) -> bool {
    columns.last().is_some_and(|over| {
        matches!(
            over.logical_type,
            LogicalType::Date | LogicalType::Timestamp { .. }
        )
    })
}

fn defect(target: &str, what: &str) -> InternalError {
    InternalError::new(STAGE, format!("the semi_additive check {target} {what}"))
}
