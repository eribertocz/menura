//! The query of the `temporal_orphans` check of a temporal relationship (VAL-2.16a): every row
//! of `from` whose keys and time are all non-NULL, and every row of `to`, each read by a `Scan`
//! with no row policy (CHK-2.2), with the columns of each role.

use menura_contracts::types::catalog::CatalogSchemaDocument;
use menura_contracts::types::common::{Dialect, LogicalType, TimestampPrecision};
use menura_model::internal_model::{
    Dataset, InternalModel, NormalizedName, OpenEnd, PopulationMode, Relationship, ValidToBound,
};
use menura_representations::check_shapes::{TemporalOrphansCheck, VersionRange};
use menura_representations::logical_plan::PlanNode;
use menura_representations::names::ColumnName;
use menura_representations::origin::Origin;

use super::key_columns::{KeyColumn, STAGE, key_columns};
use super::plan_nodes::{all_not_null, filter, scan};
use super::population_checks::declared;
use crate::compile::InternalError;
use crate::dialect_sql::{CheckShape, CheckSql, Sources, check_sql};
use crate::stop::Stop;

/// What the check reads of a temporal relationship: its datasets, its `from_time` and the
/// range of its population.
struct TemporalParts<'model> {
    from: &'model Dataset,
    to: &'model Dataset,
    from_time: &'model NormalizedName,
    valid_from: &'model NormalizedName,
    valid_to: &'model NormalizedName,
    valid_to_bound: ValidToBound,
    open_end: Option<&'model OpenEnd>,
}

/// The SQL of the `temporal_orphans` check of the temporal `relationship` in `dialect`.
pub(super) fn temporal_orphans_sql(
    dialect: Dialect,
    model: &InternalModel,
    catalog: &CatalogSchemaDocument,
    relationship: &Relationship,
) -> Result<CheckSql, Stop> {
    let parts = temporal_parts(model, relationship)?;
    let defect = |what: &str| defect(relationship, what);
    let origin = Origin::Model(relationship.location.clone());
    let (rows, from_keys, time) = from_rows(&parts, relationship, catalog, &origin)?;
    let to_keys = key_columns(parts.to, &relationship.to_columns, catalog)?;
    let bounds = key_columns(
        parts.to,
        &[parts.valid_from.clone(), parts.valid_to.clone()],
        catalog,
    )?;
    let [start, end] = bounds.as_slice() else {
        return Err(defect("has no valid_from and valid_to columns").into());
    };
    let mut version_columns = to_keys.clone();
    version_columns.extend(bounds.iter().cloned());
    let versions = scan(&parts.to.name, &version_columns, &origin);
    let check = TemporalOrphansCheck {
        rows: &rows,
        versions: &versions,
        keys: from_keys
            .iter()
            .zip(&to_keys)
            .map(|(from_key, to_key)| (from_key.scan.clone(), to_key.scan.clone()))
            .collect(),
        from_time: time.scan.clone(),
        range: VersionRange {
            valid_from: start.scan.clone(),
            valid_to: end.scan.clone(),
            valid_to_bound: parts.valid_to_bound,
            open_end: parts.open_end.cloned(),
        },
        compared_as: compared_type([&time, start, end].map(|column| &column.logical_type)),
        result_columns: result_columns(
            &from_keys,
            &time,
            &relationship.from_columns,
            parts.from_time,
        ),
    };
    check_sql(
        dialect,
        &CheckShape::TemporalOrphans(check),
        Sources { catalog, model },
    )
}

/// The rows of `from` whose keys and `from_time` are all non-NULL (VAL-2.16a), with the key
/// columns and the `from_time` column they read.
fn from_rows(
    parts: &TemporalParts<'_>,
    relationship: &Relationship,
    catalog: &CatalogSchemaDocument,
    origin: &Origin,
) -> Result<(PlanNode, Vec<KeyColumn>, KeyColumn), InternalError> {
    let from_keys = key_columns(parts.from, &relationship.from_columns, catalog)?;
    let time = key_columns(parts.from, std::slice::from_ref(parts.from_time), catalog)?
        .pop()
        .ok_or_else(|| defect(relationship, "has no from_time column"))?;
    let mut read = from_keys.clone();
    read.push(time.clone());
    let rows = filter(
        scan(&parts.from.name, &read, origin),
        all_not_null(&read)?,
        origin,
    );
    Ok((rows, from_keys, time))
}

/// The datasets, `from_time` and population range of the temporal `relationship`.
fn temporal_parts<'model>(
    model: &'model InternalModel,
    relationship: &'model Relationship,
) -> Result<TemporalParts<'model>, InternalError> {
    let temporal = relationship
        .temporal
        .as_ref()
        .ok_or_else(|| defect(relationship, "is not temporal"))?;
    let (to, population) = declared(model, &temporal.population)?;
    if to.name != relationship.to {
        return Err(defect(
            relationship,
            "names a population of another dataset",
        ));
    }
    let PopulationMode::Validity {
        valid_from,
        valid_to,
        valid_to_bound,
        open_end,
        ..
    } = &population.mode
    else {
        return Err(defect(
            relationship,
            "names a population that is not a validity one",
        ));
    };
    let from = model
        .dataset(&relationship.from)
        .ok_or_else(|| defect(relationship, "reads an unknown from dataset"))?;
    Ok(TemporalParts {
        from,
        to,
        from_time: &temporal.from_time,
        valid_from,
        valid_to,
        valid_to_bound: *valid_to_bound,
        open_end: open_end.as_ref(),
    })
}

/// A defect of the compiler about `relationship`.
fn defect(relationship: &Relationship, what: &str) -> InternalError {
    InternalError::new(
        STAGE,
        format!("the relationship {} {what}", relationship.name.as_str()),
    )
}

/// The type the three time values are compared as (VAL-2.9): E0.f checked that they are all
/// `date`, or all `timestamp` with the same `with_time_zone` (VAL-2.5); timestamps are compared
/// at the finest of their precisions.
fn compared_type(types: [&LogicalType; 3]) -> LogicalType {
    let finest = types
        .iter()
        .filter_map(|logical_type| match logical_type {
            LogicalType::Timestamp { precision, .. } => Some(*precision),
            _ => None,
        })
        .max_by_key(|precision| fraction_digits(*precision));
    match (types[0], finest) {
        (LogicalType::Timestamp { with_time_zone, .. }, Some(precision)) => {
            LogicalType::Timestamp {
                precision,
                with_time_zone: *with_time_zone,
            }
        }
        (first, _) => first.clone(),
    }
}

/// The number of fraction digits a timestamp of `precision` stores.
fn fraction_digits(precision: TimestampPrecision) -> u8 {
    match precision {
        TimestampPrecision::S => 0,
        TimestampPrecision::Ms => 3,
        TimestampPrecision::Us => 6,
        TimestampPrecision::Ns => 9,
    }
}

/// The result columns (VAL-2.16a, CHK-2.6): the `from_columns` fields in pair order, then
/// `from_time` unless it is one of them.
fn result_columns(
    from_keys: &[KeyColumn],
    time: &KeyColumn,
    from_columns: &[NormalizedName],
    from_time: &NormalizedName,
) -> Vec<(ColumnName, String)> {
    let mut columns: Vec<(ColumnName, String)> = from_keys
        .iter()
        .map(|key| (key.scan.clone(), key.result.clone()))
        .collect();
    if !from_columns.contains(from_time) {
        columns.push((time.scan.clone(), time.result.clone()));
    }
    columns
}
