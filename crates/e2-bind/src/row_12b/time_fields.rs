//! `E2-01` (E2-3.4) and `EQRY-09` (QRY-4.1): the fields of `time_grains` and `date_range`, and
//! the dimension each grain applies to.

use std::collections::BTreeMap;

use menura_contracts::types::common::{Grain, LogicalType};
use menura_model::internal_model::{FieldPath, InternalModel, ValueType};
use menura_representations::bound_query::BoundDimension;
use menura_representations::bound_query::BoundDimensionKind;
use menura_representations::origin::QueryLocation;

use crate::authorized_query::{AuthorizedQuery, ResolvedName};
use crate::error::{BindCode, BindDiagnostic};

/// `E2-01`: the first `time_grains` entry, then the date range, whose element is not a time
/// field: a field with `is_time: true` and logical kind `date` or `timestamp`.
pub(crate) fn check_time_fields(
    model: &InternalModel,
    query: &AuthorizedQuery,
) -> Result<(), BindDiagnostic> {
    let grains = query
        .time_grains
        .iter()
        .map(|grain| (&grain.target, &grain.origin));
    let range = query
        .date_range
        .iter()
        .map(|range| (&range.target, &range.origin));
    for (target, origin) in grains.chain(range) {
        if !is_time_field(model, target) {
            return Err(BindDiagnostic::new(
                BindCode::NotTimeField,
                "a time grain or date range applies only to a time field: a field with \
                 `is_time: true` whose type is a date or a timestamp"
                    .to_owned(),
                origin,
            ));
        }
    }
    Ok(())
}

fn is_time_field(model: &InternalModel, target: &ResolvedName) -> bool {
    let ResolvedName::Field(path) = target else {
        return false;
    };
    let Some(field) = model
        .dataset(&path.dataset)
        .and_then(|dataset| dataset.field(&path.field))
    else {
        return false;
    };
    let temporal = field.typed.as_ref().is_some_and(|typed| {
        matches!(
            typed.value_type,
            ValueType::Logical(LogicalType::Date | LogicalType::Timestamp { .. })
        )
    });
    field.is_time == Some(true) && temporal
}

/// `EQRY-09`: matches each `time_grains` entry to the dimensions of its field. A grain on a
/// field that is no dimension, or a second grain on one field, is reported at the entry.
pub(crate) fn assign_time_grains(
    query: &AuthorizedQuery,
) -> Result<BTreeMap<FieldPath, Grain>, BindDiagnostic> {
    let mut grains = BTreeMap::new();
    for grain in &query.time_grains {
        let field = match &grain.target {
            ResolvedName::Field(path) if is_dimension(query, path) => path,
            _ => return Err(not_dimension(&grain.origin)),
        };
        if grains.insert(field.clone(), grain.grain).is_some() {
            return Err(BindDiagnostic::new(
                BindCode::TimeGrainNotDimension,
                "this dimension already has a time grain; give each dimension at most one"
                    .to_owned(),
                &grain.origin,
            ));
        }
    }
    Ok(grains)
}

fn is_dimension(query: &AuthorizedQuery, path: &FieldPath) -> bool {
    query
        .columns
        .dimensions()
        .any(|dimension| dimension_field(dimension) == Some(path))
}

/// The field a bound dimension reads.
pub(crate) fn dimension_field(dimension: &BoundDimension) -> Option<&FieldPath> {
    match &dimension.kind {
        BoundDimensionKind::Field { field, .. } => Some(field),
        BoundDimensionKind::Expression { .. } => None,
    }
}

fn not_dimension(origin: &QueryLocation) -> BindDiagnostic {
    BindDiagnostic::new(
        BindCode::TimeGrainNotDimension,
        "a time grain applies only to a dimension of the query; add the field to `dimensions`"
            .to_owned(),
        origin,
    )
}
