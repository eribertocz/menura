//! `EQRY-11` and `EQRY-03` for the date range (QRY-4.2, QRY-2), and its bounds resolved to
//! dates.

use menura_contracts::types::query::DateBounds;
use menura_model::expression::DateValue;
use menura_representations::bound_query::BoundDateRange;

use crate::authorized_query::{AuthorizedDateRange, ResolvedName};
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

/// `EQRY-03`: a relative range needs `reference_time`.
pub(crate) fn check_reference_time(
    range: Option<&AuthorizedDateRange>,
    has_reference_time: bool,
) -> Result<(), BindDiagnostic> {
    match range {
        Some(range) if matches!(range.bounds, DateBounds::Relative(_)) && !has_reference_time => {
            Err(BindDiagnostic::new(
                BindCode::MissingReferenceTime,
                "a relative date range needs `reference_time`; add it to the query".to_owned(),
                &range.origin,
            ))
        }
        _ => Ok(()),
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
