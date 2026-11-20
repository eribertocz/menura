//! `EQRY-11` and `EQRY-03` for the date range (QRY-4.2, QRY-2), and its bounds resolved to
//! dates.

use menura_contracts::types::query::DateBounds;
use menura_model::expression::DateValue;
use menura_model::internal_model::InternalModel;
use menura_representations::bound_query::BoundDateRange;
use menura_representations::origin::QueryLocation;

use super::model_now::{first_column_reading_now, holds_now};
use crate::authorized_query::{AuthorizedDateRange, AuthorizedQuery, ResolvedName};
use crate::calendar::{parse_date, whole_units};
use crate::error::{BindCode, BindDiagnostic, BindError, InvariantViolation};

/// `EQRY-11`: an absolute range whose `from` is after its `to`.
pub(crate) fn check_order_of_bounds(range: Option<&AuthorizedDateRange>) -> Result<(), BindError> {
    let Some(range) = range else {
        return Ok(());
    };
    let DateBounds::Absolute { from, to } = &range.bounds else {
        return Ok(());
    };
    if date(from, range)? > date(to, range)? {
        return Err(BindDiagnostic::new(
            BindCode::ReversedDateRange,
            format!("the date range starts on {from}, after its end on {to}; swap `from` and `to`"),
            &range.origin,
        )
        .into());
    }
    Ok(())
}

/// `EQRY-03`: a dimension or metric whose model expression reads the current date or time, a
/// filter that reads it, then a relative range, needs `reference_time` (QRY-2, E2-5.6); the
/// first in query order is reported.
pub(crate) fn check_reference_time(
    model: &InternalModel,
    query: &AuthorizedQuery,
    has_reference_time: bool,
) -> Result<(), BindDiagnostic> {
    if has_reference_time {
        return Ok(());
    }
    if let Some(origin) = first_column_reading_now(model, &query.columns) {
        return Err(missing_reference_time(
            "this element reads the current date or time in the model",
            origin,
        ));
    }
    if let Some(filter) = query
        .filters
        .iter()
        .find(|filter| holds_now(&filter.expression))
    {
        return Err(missing_reference_time(
            "this filter reads the current date or time",
            &filter.origin,
        ));
    }
    match query.date_range.as_ref() {
        Some(range) if matches!(range.bounds, DateBounds::Relative(_)) => Err(
            missing_reference_time("a relative date range", &range.origin),
        ),
        _ => Ok(()),
    }
}

fn missing_reference_time(what: &str, origin: &QueryLocation) -> BindDiagnostic {
    BindDiagnostic::new(
        BindCode::MissingReferenceTime,
        format!("{what}, which needs `reference_time`; add it to the query"),
        origin,
    )
}

/// `E_INVALID_QUERY`: a relative range whose whole units, counted from `reference_date`,
/// leave the years 0001–9999 (QRY-4.2). A reference time whose date in `time_zone` leaves
/// those years gives no `reference_date`, and is reported the same way.
pub(crate) fn check_range_years(
    range: Option<&AuthorizedDateRange>,
    reference_date: Option<DateValue>,
) -> Result<(), BindDiagnostic> {
    let Some(range) = range else {
        return Ok(());
    };
    let DateBounds::Relative(relative) = &range.bounds else {
        return Ok(());
    };
    let include_current = relative.include_current.unwrap_or(true);
    let resolved = reference_date
        .and_then(|date| whole_units(date, relative.unit, relative.count, include_current));
    match resolved {
        Some(_) => Ok(()),
        None => Err(BindDiagnostic::new(
            BindCode::InvalidQuery,
            "this relative date range reaches a date outside the years 0001-9999; use a \
             smaller `count`"
                .to_owned(),
            &range.origin,
        )),
    }
}

/// The range with its bounds as dates: a relative range becomes the whole units of QRY-4.2,
/// counted from `reference_date`, the date of `reference_time` in `time_zone`.
pub(crate) fn resolve_date_range(
    range: &AuthorizedDateRange,
    reference_date: Option<DateValue>,
) -> Result<BoundDateRange, BindError> {
    let ResolvedName::Field(field) = &range.target else {
        return Err(invariant(range));
    };
    let (from, to) = match &range.bounds {
        DateBounds::Absolute { from, to } => (date(from, range)?, date(to, range)?),
        DateBounds::Relative(relative) => {
            let reference = reference_date.ok_or_else(|| invariant(range))?;
            let include_current = relative.include_current.unwrap_or(true);
            whole_units(reference, relative.unit, relative.count, include_current)
                .ok_or_else(|| invariant(range))?
        }
    };
    Ok(BoundDateRange {
        field: field.clone(),
        from,
        to,
        origin: range.origin.clone(),
    })
}

/// The date `text` of `range`; E1 checked its format with the JSON Schema.
fn date(text: &str, range: &AuthorizedDateRange) -> Result<DateValue, BindError> {
    parse_date(text).ok_or_else(|| invariant(range))
}

fn invariant(range: &AuthorizedDateRange) -> BindError {
    BindError::invariant(InvariantViolation::UnresolvableDateRange, &range.origin)
}
